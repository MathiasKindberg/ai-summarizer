pub(crate) mod summarizer;
pub(crate) mod title_categorical_scoring;
pub(crate) mod title_numerical_scoring;

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub(crate) enum Category {
    High,
    Medium,
    Low,
    Zero,
}

pub(crate) fn convert_titles_to_messages(
    titles: Vec<String>,
    system_prompt: &str,
    response_schema: Schema,
) -> Vec<crate::openai::OpenAIChatCompletionQuery> {
    titles
        .chunks(crate::config::CONFIG.title_scorer.titles_scored_per_request)
        .map(|titles| {
            vec![
                crate::openai::Message {
                    role: crate::openai::Role::Developer,
                    content: system_prompt.to_string(),
                },
                crate::openai::Message {
                    role: crate::openai::Role::User,
                    content: serde_json::to_string(titles).unwrap(),
                },
            ]
        })
        .map(|messages| {
            crate::openai::OpenAIChatCompletionQuery::new(messages, response_schema.clone())
        })
        .collect()
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct OpenAIChatCompletionQuery {
    model: String,
    messages: Vec<Message>,
    response_format: ResponseFormat,
}

impl OpenAIChatCompletionQuery {
    pub(crate) fn new(messages: Vec<Message>, schema: Schema) -> Self {
        Self {
            model: crate::config::CONFIG.title_scorer.model.clone(),
            messages,
            response_format: ResponseFormat {
                response_type: "json_schema".to_string(),
                json_schema: schema,
            },
        }
    }

    pub(crate) fn system_prompt_and_content_to_messages(
        system_prompt: &str,
        content: &str,
    ) -> Vec<Message> {
        vec![
            Message {
                role: Role::Developer,
                content: system_prompt.to_string(),
            },
            Message {
                role: Role::User,
                content: content.to_string(),
            },
        ]
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Role {
    Developer,
    User,
    Assistant,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct Message {
    pub(crate) role: Role,
    pub(crate) content: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct Schema {
    pub(crate) name: String,
    pub(crate) schema: serde_json::Value,
    pub(crate) strict: bool,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct ResponseFormat {
    #[serde(rename = "type")]
    response_type: String,
    json_schema: Schema,
}

// Open AI Generic Queries
#[derive(Debug, serde::Deserialize)]
pub(crate) struct OpenAIChatCompletionResponse {
    #[allow(unused)]
    pub(crate) id: String,
    #[allow(unused)]
    pub(crate) object: String,
    #[allow(unused)]
    pub(crate) created: i64,
    #[allow(unused)]
    pub(crate) model: String,
    pub(crate) choices: Vec<Choice>,
    #[allow(unused)]
    pub(crate) usage: Usage,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct ResponseMessage {
    pub(crate) role: Role,
    pub(crate) content: String,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct Choice {
    pub(crate) message: ResponseMessage,
    #[allow(unused)]
    pub(crate) finish_reason: String,
    #[allow(unused)]
    pub(crate) index: i64,
}

#[derive(Debug, serde::Deserialize)]
#[allow(unused)]
pub(crate) struct Usage {
    prompt_tokens: i64,
    completion_tokens: i64,
    total_tokens: i64,
}
