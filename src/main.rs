// struct Args {
//     date: String,
// }

pub(crate) mod config;
pub(crate) mod google_chat;
pub(crate) mod hn_api;
pub(crate) mod openai;
pub(crate) mod scraper;

pub(crate) static CLIENT: std::sync::LazyLock<reqwest::Client> =
    std::sync::LazyLock::new(|| reqwest::Client::new());

#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "categorical")]
    mode: Mode,

    #[arg(short, long, default_value = "false")]
    score_titles: bool,
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

#[derive(Debug, serde::Deserialize, Clone)]
struct Story {
    #[allow(unused)]
    by: String,
    #[allow(unused)]
    score: i64,

    title: String,
    url: Option<String>,
    #[serde(rename = "type")]
    story_type: String,

    // Not included in json response. Our own enrichment.
    ai_impact_score: Option<ImpactScore>,
    text: Option<String>,
    summary: Option<Vec<String>>,
}

#[derive(Debug, serde::Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
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

// async fn remove_low_scored_stories(stories: Vec<Story>) -> Vec<Story> {
//     let impact_and_title: Vec<(String, &String)> = stories
//         .iter()
//         .map(|story| {
//             (
//                 match story.ai_impact_score.as_ref().unwrap() {
//                     ImpactScore::Numerical(score) => score.to_string(),
//                     ImpactScore::Categorical(score) => format!("{:?}", score),
//                 },
//                 &story.title,
//             )
//         })
//         .collect();

//     println!("Impact | Title");
//     impact_and_title
//         .iter()
//         .for_each(|(impact, title)| println!("{:>3} | {}", impact, title));

//     stories
//         .into_iter()
//         .filter(|score| match score.ai_impact_score.as_ref().unwrap() {
//             ImpactScore::Numerical(score) => score >= &config::CONFIG.min_ai_impact_score,
//             ImpactScore::Categorical(category) => matches!(
//                 category,
//                 crate::openai::Category::High | crate::openai::Category::Medium
//             ),
//         })
//         .collect()
// }

async fn summarize_and_score_scraped_stories(stories: Vec<Story>) -> Vec<Story> {
    let mut enriched_stories = Vec::with_capacity(stories.len());

    for story in stories {
        let summary = crate::openai::summarizer::enrich_story(story)
            .await
            .unwrap();

        enriched_stories.push(summary);
    }

    enriched_stories
}

async fn get_summary(args: Args) {
    tracing::info!(
        config =? config::CONFIG,
        args =? args,
        "Started AI summarizer"
    );
    let stories = get_hackernews_top_stories().await;

    tracing::info!(num_stories = stories.len(), "Got top stories");

    let num_stories = stories.len();
    let stories = remove_job_adverts(stories);
    tracing::info!(
        num_adverts_removed = num_stories - stories.len(),
        "Removed job adverts"
    );

    let stories = scraper::enrich_stories(stories).await;

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

    let mut summaries = summarize_and_score_scraped_stories(stories).await;

    summaries.sort_by(|a, b| {
        b.ai_impact_score
            .as_ref()
            .unwrap()
            .cmp(a.ai_impact_score.as_ref().unwrap())
    });
    summaries.reverse();

    let summaries = &summaries[..config::CONFIG.max_number_of_stories_to_present];

    println!("{:#?}", summaries);

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
