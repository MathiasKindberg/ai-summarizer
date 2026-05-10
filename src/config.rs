pub(crate) struct Config {
    pub(crate) api_key: String,
    pub(crate) model: String,
    pub(crate) reasoning_effort: Option<crate::openai::ReasoningEffort>,
    pub(crate) system_prompt: String,

    pub(crate) num_titles_to_request: usize,
    pub(crate) google_chat_webhook_url: String,
    pub(crate) max_number_of_stories_to_present: usize,
    pub(crate) log_to_console: bool,
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let api_key = if self.api_key.len() > 12 {
            format!(
                "{}…{}",
                &self.api_key[..8],
                &self.api_key[self.api_key.len() - 4..]
            )
        } else {
            "***".to_string()
        };
        let webhook = match self.google_chat_webhook_url.split_once('?') {
            Some((path, _)) => format!("{path}?<redacted>"),
            None => "***".to_string(),
        };

        f.debug_struct("Config")
            .field("api_key", &api_key)
            .field("model", &self.model)
            .field("reasoning_effort", &self.reasoning_effort)
            .field("system_prompt", &self.system_prompt)
            .field("num_titles_to_request", &self.num_titles_to_request)
            .field("google_chat_webhook_url", &webhook)
            .field(
                "max_number_of_stories_to_present",
                &self.max_number_of_stories_to_present,
            )
            .field("log_to_console", &self.log_to_console)
            .finish()
    }
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
        reasoning_effort: std::env::var("OPENAI_REASONING_EFFORT")
            .ok()
            .map(|v| v.parse().expect("OPENAI_REASONING_EFFORT invalid")),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config() -> Config {
        Config {
            api_key: "sk-proj-AAAABBBBCCCCDDDD".to_string(),
            model: "o3-mini-2025-01-31".to_string(),
            reasoning_effort: None,
            system_prompt: "be brief".to_string(),
            num_titles_to_request: 60,
            google_chat_webhook_url:
                "https://chat.googleapis.com/v1/spaces/XYZ/messages?key=SECRET&token=ALSO"
                    .to_string(),
            max_number_of_stories_to_present: 5,
            log_to_console: false,
        }
    }

    #[test]
    fn debug_redacts_api_key() {
        let rendered = format!("{:?}", sample_config());
        assert!(rendered.contains("sk-proj-…DDDD"), "got: {rendered}");
        assert!(!rendered.contains("AAAABBBBCCCC"), "got: {rendered}");
    }

    #[test]
    fn debug_redacts_webhook_query_string() {
        let rendered = format!("{:?}", sample_config());
        assert!(
            rendered.contains("https://chat.googleapis.com/v1/spaces/XYZ/messages?<redacted>"),
            "got: {rendered}"
        );
        assert!(!rendered.contains("SECRET"), "got: {rendered}");
        assert!(!rendered.contains("ALSO"), "got: {rendered}");
    }

    // Regression guard: if a new field is added to `Config`, the exhaustive
    // destructure below will fail to compile, and the field-name asserts will
    // fail at runtime — both force whoever adds the field to update the
    // manual `Debug` impl above (and decide whether the new field needs
    // redaction).
    #[test]
    fn debug_covers_every_field() {
        let cfg = sample_config();
        let Config {
            api_key,
            model,
            reasoning_effort,
            system_prompt,
            num_titles_to_request,
            google_chat_webhook_url,
            max_number_of_stories_to_present,
            log_to_console,
        } = &cfg;
        let _ = (
            api_key,
            model,
            reasoning_effort,
            system_prompt,
            num_titles_to_request,
            google_chat_webhook_url,
            max_number_of_stories_to_present,
            log_to_console,
        );

        let rendered = format!("{cfg:?}");
        for field in [
            "api_key",
            "model",
            "reasoning_effort",
            "system_prompt",
            "num_titles_to_request",
            "google_chat_webhook_url",
            "max_number_of_stories_to_present",
            "log_to_console",
        ] {
            assert!(rendered.contains(field), "missing {field} in: {rendered}");
        }
    }
}
