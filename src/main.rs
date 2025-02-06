pub(crate) mod config;
pub(crate) mod db;
pub(crate) mod google_chat;
pub(crate) mod hn_api;
pub(crate) mod openai;
pub(crate) mod scraper;

pub(crate) static CLIENT: std::sync::LazyLock<reqwest::Client> =
    std::sync::LazyLock::new(reqwest::Client::new);

#[derive(Debug, Clone, clap::Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "categorical")]
    mode: Mode,

    #[arg(short, long, default_value = "false")]
    score_titles: bool,

    #[arg(short, long, default_value = "false")]
    export_text: bool,

    #[arg(short, long, default_value = "false")]
    reset: bool,

    #[arg(short, long, default_value = "false")]
    no_cron: bool,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum Mode {
    Categorical,
    Numerical,
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
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
    ai_impact_score: Option<ImpactScore>,
    text: Option<String>,
    summary: Option<Vec<String>>,

    // Statistics
    usage: Option<crate::openai::Usage>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Deserialize, serde::Serialize)]
enum ImpactScore {
    Numerical(i64),
    Categorical(crate::openai::Category),
}

impl std::fmt::Display for ImpactScore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImpactScore::Numerical(n) => write!(f, "{}", n),
            ImpactScore::Categorical(c) => write!(f, "{}", c),
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

async fn score_impact_of_numerical_stories(stories: Vec<Story>) -> Vec<Story> {
    let titles = stories
        .iter()
        .map(|s| s.title.clone())
        .collect::<Vec<String>>();

    let ai_impact_scores = crate::openai::title_numerical_scoring::score_ai_impact(titles)
        .await
        .unwrap();

    stories
        .into_iter()
        .zip(ai_impact_scores.into_iter())
        .map(|(mut story, score)| {
            story.ai_impact_score = Some(ImpactScore::Numerical(score));
            story
        })
        .collect()
}

async fn score_impact_of_categorical_stories(stories: Vec<Story>) -> Vec<Story> {
    let titles = stories
        .iter()
        .map(|s| s.title.clone())
        .collect::<Vec<String>>();

    let ai_impact_scores = crate::openai::title_categorical_scoring::score_ai_impact(titles)
        .await
        .unwrap();

    stories
        .into_iter()
        .zip(ai_impact_scores.into_iter())
        .map(|(mut story, score)| {
            story.ai_impact_score = Some(ImpactScore::Categorical(score));
            story
        })
        .collect()
}

async fn summarize_and_score_scraped_stories(stories: Vec<Story>) -> Vec<Story> {
    let mut join_set: tokio::task::JoinSet<anyhow::Result<Story>> = tokio::task::JoinSet::new();
    let mut enriched_stories = Vec::with_capacity(stories.len());

    for story in stories {
        let url = story.url.clone().unwrap();
        join_set.spawn(async move {
            let story = crate::openai::summarizer::enrich_story(story)
                .await?;
            tracing::info!(title = story.title, url = url, usage =? story.usage.clone().expect("Usage"), "Scored and summarized story");
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

    enriched_stories
}

async fn get_summary(args: Args) -> anyhow::Result<()> {
    let db = db::open_db(args.reset)?;

    tracing::info!("Database opened");
    let processed_stories: Vec<i64> = db::get_processed_stories(&db)?;

    tracing::info!(
        num_processed_stories = processed_stories.len(),
        "Got already processed stories"
    );

    let stories = hn_api::get_hackernews_top_stories()
        .await
        .expect("Failed to get top stories");

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

    let stories = scraper::enrich_stories(stories, args.export_text)
        .await
        .expect("Failed to enrich stories");

    tracing::info!(
        num_scraped_stories = stories.len(),
        "Finished scraping stories"
    );

    if args.score_titles {
        let _ = match args.mode {
            Mode::Categorical => score_impact_of_categorical_stories(stories.clone()).await,
            Mode::Numerical => score_impact_of_numerical_stories(stories.clone()).await,
        };
    }

    let mut stories = summarize_and_score_scraped_stories(stories).await;

    stories.sort_by(|a, b| {
        b.ai_impact_score
            .as_ref()
            .unwrap()
            .cmp(a.ai_impact_score.as_ref().unwrap())
    });
    stories.reverse();

    let stories = stories[..config::config()
        .max_number_of_stories_to_present
        .min(stories.len())]
        .to_vec();

    if stories.is_empty() {
        tracing::info!("No stories to present");
        return Ok(());
    }

    if args.export_text {
        let json_summaries = serde_json::to_string_pretty(&stories).unwrap();
        std::fs::write("src/examples/stories.json", json_summaries)
            .expect("Failed to write to file");
    }

    let message = google_chat::create_message(stories.clone());
    google_chat::send_message(message, &config::config().google_chat_webhook_url)
        .await
        .expect("Failed to send message");
    tracing::info!("Sent message to google chat");

    db::insert_stories(&db, stories)?;
    tracing::info!("Inserted stories into db");

    Ok(())
}

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(tracing::Level::INFO)
        // completes the builder.
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    use clap::Parser;
    let args = Args::parse();

    let (tx, mut rx) = tokio::sync::mpsc::channel(1);

    ctrlc::set_handler(move || {
        tx.blocking_send(())
            .expect("Could not send signal on channel.")
    })
    .expect("Error setting Ctrl-C handler");

    // Make sure to run program in separate task to ensure we don't hit
    // tokio main thread weirdness.
    tokio::spawn(async move {
        if args.no_cron {
            tracing::info!(
                config =? crate::config::config(),
                args =? args,
                "Starting AI summarizer on demand"
            );
            let _ = get_summary(args).await.map_err(|err| tracing::error!(error =? err, "Failed to get summary"));
        } else {
            tracing::info!(
                config =? crate::config::config(),
                args =? args,
                "Starting AI summarizer on cron schedule"
            );
            let scheduler = tokio_cron_scheduler::JobScheduler::new()
                .await
                .expect("Failed to create scheduler");

            scheduler
                .add(
                    tokio_cron_scheduler::Job::new_async(
                        crate::config::config().cron_schedule.clone(),
                        move |_, _| {
                            Box::pin({
                                let args = args.clone();
                                async move {
                                    let _ = get_summary(args).await.map_err(|err| tracing::error!(error =? err, "Failed to get summary"));
                                }
                            })  
                        },
                    )
                    .expect("Failed to add job"),
                )
                .await
                .expect("Failed to add job");
            scheduler.start().await.expect("Failed to start scheduler");
            
            rx.recv().await.expect("Failed to receive signal");
            tracing::info!("Ctrl-C received: Shutting down");
        }
    })
    .await
    .expect("Program to gracefully run");
    tracing::info!("Exiting");
}
