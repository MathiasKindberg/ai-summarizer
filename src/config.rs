#[derive(Debug)]
pub(crate) struct Config {
    pub(crate) title_scorer: TitleScorer,
    pub(crate) summarizer: Summarizer,
    pub(crate) num_titles_to_request: usize,
    pub(crate) google_chat_test_webhook_url: String,
    pub(crate) max_number_of_stories_to_present: usize,
}

pub(crate) static CONFIG: std::sync::LazyLock<Config> = std::sync::LazyLock::new(|| {
    dotenvy::dotenv().expect("Failed to load .env file");

    Config {
        title_scorer: TitleScorer::new(),
        summarizer: Summarizer::new(),

        num_titles_to_request: std::env::var("NUM_TITLES_TO_REQUEST")
            .expect("NUM_TITLES_TO_REQUEST not set")
            .parse()
            .unwrap(),
        google_chat_test_webhook_url: std::env::var("GOOGLE_CHAT_TEST_WEBHOOK_URL")
            .expect("GOOGLE_CHAT_TEST_WEBHOOK_URL not set"),
        max_number_of_stories_to_present: std::env::var("MAX_NUMBER_OF_STORIES_TO_PRESENT")
            .expect("MAX_NUMBER_OF_STORIES_TO_PRESENT not set")
            .parse()
            .unwrap(),
    }
});

#[derive(Debug)]
pub(crate) struct TitleScorer {
    pub(crate) api_key: String,
    pub(crate) model: String,
    pub(crate) categorical_system_prompt: String,
    pub(crate) numerical_system_prompt: String,
    pub(crate) titles_scored_per_request: usize,
}

impl TitleScorer {
    fn new() -> Self {
        Self {
            api_key: std::env::var("OPENAI_TITLE_SCORER_API_KEY")
                .expect("OPENAI_TITLE_SCORER_API_KEY not set"),
            model: std::env::var("OPENAI_TITLE_SCORER_MODEL")
                .expect("OPENAI_TITLE_SCORER_MODEL not set"),
            categorical_system_prompt: std::env::var("TITLE_SCORER_CATEGORICAL_SYSTEM_PROMPT")
                .expect("TITLE_SCORER_CATEGORICAL_SYSTEM_PROMPT not set"),
            numerical_system_prompt: std::env::var("TITLE_SCORER_NUMERICAL_SYSTEM_PROMPT")
                .expect("TITLE_SCORER_NUMERICAL_SYSTEM_PROMPT not set"),
            titles_scored_per_request: std::env::var("TITLES_SCORED_PER_REQUEST")
                .expect("TITLES_SCORED_PER_REQUEST not set")
                .parse()
                .unwrap(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct Summarizer {
    pub(crate) api_key: String,
    pub(crate) model: String,
    pub(crate) numerical_system_prompt: String,
    pub(crate) categorical_system_prompt: String,
}

impl Summarizer {
    fn new() -> Self {
        Self {
            api_key: std::env::var("OPENAI_SUMMARIZER_API_KEY")
                .expect("OPENAI_SUMMARIZER_API_KEY not set"),
            model: std::env::var("OPENAI_SUMMARIZER_MODEL")
                .expect("OPENAI_SUMMARIZER_MODEL not set"),
            numerical_system_prompt: std::env::var("SUMMARIZER_NUMERICAL_SYSTEM_PROMPT")
                .expect("SUMMARIZER_NUMERICAL_SYSTEM_PROMPT not set"),
            categorical_system_prompt: std::env::var("SUMMARIZER_CATEGORICAL_SYSTEM_PROMPT")
                .expect("SUMMARIZER_CATEGORICAL_SYSTEM_PROMPT not set"),
        }
    }
}
