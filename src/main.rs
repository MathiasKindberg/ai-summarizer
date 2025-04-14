use tracing_subscriber::util::SubscriberInitExt;

mod backoff;
pub(crate) mod config;
pub(crate) mod db;
pub(crate) mod google_chat;
pub(crate) mod hn_api;
mod lints;
pub(crate) mod openai;
pub(crate) mod scraper;
pub(crate) static CLIENT: std::sync::LazyLock<reqwest::Client> =
    std::sync::LazyLock::new(reqwest::Client::new);

#[derive(Debug, Clone, clap::Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "false")]
    #[arg(help = "Export the stories to json in the export directory")]
    export_text: bool,

    #[arg(short, long, default_value = "false")]
    #[arg(help = "Reset the database")]
    reset: bool,

    #[arg(short, long, default_value = "false")]
    #[arg(help = "Log to console")]
    log_to_console: bool,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct Story {
    id: i64,
    score: i64,
    descendants: Option<i64>,

    title: String,

    url: Option<String>,

    #[serde(rename = "type")]
    story_type: String,

    // Not included in json response. Our own enrichment.
    ai_impact_score: Option<crate::openai::Category>,
    text: Option<String>,
    summary: Option<Vec<String>>,

    // Statistics
    usage: Option<crate::openai::Usage>,
}

impl Default for Story {
    fn default() -> Self {
        Self {
            id: 0,
            score: 0,
            descendants: None,
            title: "".to_string(),
            url: None,
            story_type: "".to_string(),
            ai_impact_score: None,
            text: None,
            summary: None,
            usage: None,
        }
    }
}

fn remove_job_adverts(stories: Vec<Story>) -> Vec<Story> {
    stories
        .into_iter()
        .filter(|s| s.story_type != "job")
        .collect()
}

fn remove_stories_without_url(stories: Vec<Story>) -> Vec<Story> {
    stories.into_iter().filter(|s| s.url.is_some()).collect()
}

async fn summarize_and_score_scraped_stories(stories: Vec<Story>) -> anyhow::Result<Vec<Story>> {
    let mut join_set: tokio::task::JoinSet<anyhow::Result<Story>> = tokio::task::JoinSet::new();
    let mut enriched_stories = Vec::with_capacity(stories.len());

    for story in stories {
        let url = story.url.clone().unwrap();
        join_set.spawn(async move {
            let story = crate::openai::enrich_story(story).await?;
            tracing::info!(
                title = story.title,
                url = url,
                ai_score =? story.ai_impact_score,
                votes = story.score,
                usage =? story.usage,
                "Scored and summarized story"
            );
            Ok(story)
        });
    }

    while let Some(result) = join_set.join_next().await {
        match result.expect("JoinSet to work") {
            Ok(story) => enriched_stories.push(story),
            Err(e) => tracing::error!(error =? e, "Error enriching story"),
        }
    }

    let total_usage = crate::openai::Usage {
        prompt_tokens: enriched_stories
            .iter()
            .map(|s| s.usage.as_ref().expect("Usage").prompt_tokens)
            .sum(),
        completion_tokens: enriched_stories
            .iter()
            .map(|s| s.usage.as_ref().expect("Usage").completion_tokens)
            .sum(),
        total_tokens: enriched_stories
            .iter()
            .map(|s| s.usage.as_ref().expect("Usage").total_tokens)
            .sum(),
    };

    tracing::info!(
        num_stories = enriched_stories.len(),
        total_usage =? total_usage,
        "Finished enriching stories"
    );

    Ok(enriched_stories)
}

fn sort_stories(stories: &mut [Story]) {
    stories.sort_by(|a, b| {
        let a_score = a.ai_impact_score.as_ref().unwrap();
        let b_score = b.ai_impact_score.as_ref().unwrap();

        if a_score == b_score {
            b.score.cmp(&a.score)
        } else {
            a_score.cmp(b_score)
        }
    })
}

async fn get_summary(args: Args) -> anyhow::Result<()> {
    let db = db::open_db(args.reset)?;

    tracing::info!("Database opened");
    let processed_stories: Vec<i64> = db::get_processed_stories(&db)?;

    tracing::info!(
        num_processed_stories = processed_stories.len(),
        "Got already processed stories"
    );

    let stories = hn_api::get_hackernews_top_stories().await?;

    tracing::info!(num_stories = stories.len(), "Got top stories");

    let num_stories = stories.len();
    let stories = remove_job_adverts(stories);
    tracing::info!(
        num_adverts_removed = num_stories - stories.len(),
        "Removed job adverts"
    );

    let num_stories = stories.len();
    let stories = remove_stories_without_url(stories);
    tracing::info!(
        num_stories_without_url_removed = num_stories - stories.len(),
        "Removed stories without url"
    );

    let num_stories = stories.len();
    let stories: Vec<_> = stories
        .into_iter()
        .filter(|s| !processed_stories.contains(&s.id))
        .collect();

    tracing::info!(
        num_stories_filtered_out = num_stories - stories.len(),
        "Filtered out already processed stories"
    );

    let stories = scraper::enrich_stories(stories, args.export_text).await?;

    tracing::info!(
        num_scraped_stories = stories.len(),
        "Finished scraping stories"
    );

    let mut stories = summarize_and_score_scraped_stories(stories).await?;

    sort_stories(&mut stories);

    let mut stories = stories[..config::config()
        .max_number_of_stories_to_present
        .min(stories.len())]
        .to_vec();

    stories.retain(|s| {
        s.ai_impact_score.as_ref().unwrap() == &crate::openai::Category::High && s.summary.is_some()
    });

    if stories.is_empty() {
        tracing::info!("No stories to send to google chat");
        return Ok(());
    }

    if args.export_text {
        let json_summaries = serde_json::to_string_pretty(&stories)?;
        std::fs::create_dir_all("export")?;
        std::fs::write("export/exported_stories.json", json_summaries)?;
        tracing::info!("Exported stories to export/exported_stories.json");
    }

    let message = google_chat::create_message(stories.clone())?;
    google_chat::send_message(message, &config::config().google_chat_webhook_url).await?;
    tracing::info!("Sent message to google chat");

    db::insert_stories(&db, &stories)?;
    tracing::info!(
        num = stories.len(),
        ids =? stories.iter().map(|s: &Story| s.id).collect::<Vec<_>>(),
        "Inserted stories into db"
    );

    Ok(())
}

#[tokio::main]
async fn main() {
    use tracing_subscriber::layer::Layer;
    use tracing_subscriber::layer::SubscriberExt;

    use clap::Parser;
    let args = Args::parse();

    let file_appender = tracing_appender::rolling::daily("./log", "ai_summarizer.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let file_layer = tracing_subscriber::fmt::layer();
    let file_layer = file_layer
        .with_writer(non_blocking)
        .json()
        .with_filter(tracing::level_filters::LevelFilter::INFO)
        .boxed();

    let pretty_layer = tracing_subscriber::fmt::layer()
        .with_file(true)
        .with_line_number(true)
        .with_writer(std::io::stdout)
        .with_filter(tracing::level_filters::LevelFilter::INFO)
        .boxed();

    let registry = tracing_subscriber::registry().with(file_layer);

    if config::config().log_to_console || args.log_to_console {
        registry.with(pretty_layer).init();
    } else {
        registry.init();
    };

    tracing::info!(
        config =? config::config(),
        args =? args,
        "Starting AI Summarizer"
    );

    // Timeout after an hour
    const TIMEOUT: u64 = 60 * 60 * 60;
    tokio::select! {
        res = get_summary(args) => match res {
            Ok(_) => tracing::info!("AI Summarizer finished"),
            Err(e) => tracing::error!(error =? e, "Error when getting summary"),
        },
        _ = tokio::time::sleep(std::time::Duration::from_secs(TIMEOUT)) => {
            tracing::error!(timeout = TIMEOUT, "Timeout when getting summary");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_stories() {
        let mut stories = vec![
            Story {
                id: 0,
                score: 200,
                ai_impact_score: Some(crate::openai::Category::High),
                ..Default::default()
            },
            Story {
                id: 1,
                score: 100,
                ai_impact_score: Some(crate::openai::Category::Medium),
                ..Default::default()
            },
            Story {
                id: 2,
                score: 0,
                ai_impact_score: Some(crate::openai::Category::Low),
                ..Default::default()
            },
            Story {
                id: 3,
                score: 400,
                ai_impact_score: Some(crate::openai::Category::High),
                ..Default::default()
            },
            Story {
                id: 4,
                score: 300,
                ai_impact_score: Some(crate::openai::Category::Medium),
                ..Default::default()
            },
            Story {
                id: 5,
                score: 300,
                ai_impact_score: Some(crate::openai::Category::High),
                ..Default::default()
            },
        ];

        sort_stories(&mut stories);

        // Stories should be sorted by impact score (High > Medium > Low > Zero)
        // Within the same impact score, higher HN score should come first
        assert_eq!(stories[0].id, 3); // High impact
        assert_eq!(stories[1].id, 5); // High impact
        assert_eq!(stories[2].id, 0); // High impact
        assert_eq!(stories[3].id, 4); // Medium impact
        assert_eq!(stories[4].id, 1); // Medium impact
        assert_eq!(stories[5].id, 2); // Low impact
    }
}
