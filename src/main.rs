// struct Args {
//     date: String,
// }

pub(crate) mod config;
pub(crate) mod queries;
pub(crate) mod scraper;

pub(crate) static CLIENT: std::sync::LazyLock<reqwest::Client> =
    std::sync::LazyLock::new(|| reqwest::Client::new());

#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "categorical")]
    mode: Mode,
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

#[derive(Debug, serde::Deserialize)]
struct Story {
    by: String,
    score: i64,
    title: String,
    url: Option<String>,
    #[serde(rename = "type")]
    story_type: String,

    // Not included in json response. Our own enrichment.
    ai_impact_score: Option<ImpactScore>,
}

#[derive(Debug, serde::Deserialize)]
enum ImpactScore {
    Numerical(i64),
    Categorical(crate::queries::title_categorical_scoring::Category),
}

impl std::fmt::Display for ImpactScore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImpactScore::Numerical(n) => write!(f, "{}", n),
            ImpactScore::Categorical(c) => write!(f, "{}", c),
        }
    }
}

async fn get_hackernews_top_stories() -> Vec<Story> {
    let response = CLIENT
        .get("https://hacker-news.firebaseio.com/v0/topstories.json")
        .send()
        .await
        .unwrap();
    let stories: Vec<i64> =
        response.json::<Vec<i64>>().await.unwrap()[..config::CONFIG.num_titles_to_request].to_vec();

    // We can do this in parallel but this is good enough for now.
    let mut enriched_stories = Vec::with_capacity(stories.len());

    let mut queries_set = tokio::task::JoinSet::new();
    for story in stories {
        queries_set.spawn(async move {
            CLIENT
                .get(format!(
                    "https://hacker-news.firebaseio.com/v0/item/{}.json",
                    story
                ))
                .send()
                .await
                .unwrap()
                .json::<Story>()
                .await
                .unwrap()
        });
    }

    while let Some(res) = queries_set.join_next().await {
        enriched_stories.push(res.unwrap());
    }

    enriched_stories
}

fn remove_job_adverts(stories: Vec<Story>) -> Vec<Story> {
    stories
        .into_iter()
        .filter(|s| s.story_type != "job")
        .collect()
}

async fn score_impact_of_numerical_stories(stories: Vec<Story>) -> Vec<Story> {
    let titles = stories
        .iter()
        .map(|s| s.title.clone())
        .collect::<Vec<String>>();

    let ai_impact_scores = crate::queries::title_numerical_scoring::score_ai_impact(&titles)
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

    let ai_impact_scores = crate::queries::title_categorical_scoring::score_ai_impact(&titles)
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

async fn remove_low_scored_stories(stories: Vec<Story>) -> Vec<Story> {
    let impact_and_title: Vec<(String, &String)> = stories
        .iter()
        .map(|story| {
            (
                match story.ai_impact_score.as_ref().unwrap() {
                    ImpactScore::Numerical(score) => score.to_string(),
                    ImpactScore::Categorical(score) => format!("{:?}", score),
                },
                &story.title,
            )
        })
        .collect();

    println!("Impact | Title");
    impact_and_title
        .iter()
        .for_each(|(impact, title)| println!("{:>3} | {}", impact, title));

    stories
        .into_iter()
        .filter(|score| match score.ai_impact_score.as_ref().unwrap() {
            ImpactScore::Numerical(score) => score >= &config::CONFIG.min_ai_impact_score,
            ImpactScore::Categorical(category) => matches!(
                category,
                crate::queries::title_categorical_scoring::Category::High
                    | crate::queries::title_categorical_scoring::Category::Medium
            ),
        })
        .collect()
}

async fn scrape_stories(stories: Vec<Story>) -> Vec<String> {
    let mut scraped_stories = Vec::with_capacity(stories.len());

    let mut queries_set: tokio::task::JoinSet<anyhow::Result<String>> = tokio::task::JoinSet::new();

    for story in stories {
        queries_set.spawn(async move {
            let raw_text = crate::scraper::scrape_text(&story.url.unwrap()).await?;
            let trimmed_text = crate::scraper::html_to_trimmed_text(&raw_text)?;

            use std::io::Write;
            let mut file =
                std::fs::File::create(format!("tmp/{}.txt", story.title.replace(" ", "-")))?;
            file.write_all(trimmed_text.as_bytes())?;

            Ok(trimmed_text)
        });
    }

    while let Some(res) = queries_set.join_next().await {
        match res.expect("Joinset to work") {
            Ok(text) => scraped_stories.push(text),
            Err(e) => tracing::error!(error =? e, "Error scraping story"),
        }
    }

    scraped_stories
}

async fn get_summary(args: Args) {
    tracing::info!(
        config =? config::CONFIG,
        args =? args,
        "Getting top stories"
    );
    let stories = get_hackernews_top_stories().await;

    tracing::info!(num_stories = stories.len(), "Removing job adverts");
    let stories = remove_job_adverts(stories);

    let scraped_stories = scrape_stories(stories).await;

    // tracing::info!(
    //     num_stories = stories.len(),
    //     "Removing stories not related to ai"
    // );

    // let stories = match args.mode {
    //     Mode::Categorical => score_impact_of_categorical_stories(stories).await,
    //     Mode::Numerical => score_impact_of_numerical_stories(stories).await,
    // };

    // let stories = remove_low_scored_stories(stories).await;

    // tracing::info!(num_stories = stories.len(), "Stories with ai impact");

    // stories.iter().for_each(|story| {
    //     println!(
    //         "{} | {}",
    //         story.ai_impact_score.as_ref().unwrap(),
    //         story.title
    //     )
    // });
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

    // Make sure to run program in separate task to ensure we don't hit
    // tokio main thread weirdness.
    tokio::spawn(get_summary(args)).await.unwrap();
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn test_get_summary() {}
}
