#[derive(Debug)]
pub(crate) struct Config {
    pub(crate) api_key: String,
    pub(crate) model: String,
    pub(crate) system_prompt: String,

    pub(crate) num_titles_to_request: usize,
    pub(crate) google_chat_webhook_url: String,
    pub(crate) max_number_of_stories_to_present: usize,
    pub(crate) log_to_console: bool,
}

static CONFIG: std::sync::LazyLock<Config> = std::sync::LazyLock::new(|| {
    match dotenvy::dotenv() {
        Ok(_) => (),
        Err(e) => tracing::error!(err =? e,
            "Failed to load .env file. Continuing with default values."),
    }

    Config {
        api_key: std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set"),
        model: std::env::var("OPENAI_MODEL").expect("OPENAI_MODEL not set"),
        system_prompt: std::env::var("SYSTEM_PROMPT").expect("SYSTEM_PROMPT not set"),

        num_titles_to_request: std::env::var("NUM_TITLES_TO_REQUEST")
            .unwrap_or("60".to_string())
            .parse()
            .unwrap(),
        google_chat_webhook_url: std::env::var("GOOGLE_CHAT_WEBHOOK_URL")
            .expect("GOOGLE_CHAT_WEBHOOK_URL not set"),
        max_number_of_stories_to_present: std::env::var("MAX_NUMBER_OF_STORIES_TO_PRESENT")
            .unwrap_or("5".to_string())
            .parse()
            .unwrap(),
        log_to_console: std::env::var("LOG_TO_CONSOLE")
            .expect("LOG_TO_CONSOLE not set")
            .parse()
            .unwrap(),
    }
});

pub(crate) fn config() -> &'static Config {
    &CONFIG
}
