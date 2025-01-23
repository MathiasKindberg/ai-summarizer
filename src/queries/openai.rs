pub(crate) fn convert_titles_to_messages(
    titles: &Vec<String>,
    system_prompt: &str,
    response_schema: Schema,
) -> Vec<crate::queries::openai::OpenAIChatCompletionQuery> {
    titles
        .chunks(crate::config::CONFIG.title_scorer.titles_scored_per_request)
        .map(|titles| {
            vec![
                crate::queries::openai::Message {
                    role: crate::queries::openai::Role::Developer,
                    content: system_prompt.to_string(),
                },
                crate::queries::openai::Message {
                    role: crate::queries::openai::Role::User,
                    content: serde_json::to_string(titles).unwrap(),
                },
            ]
        })
        .map(|messages| {
            crate::queries::openai::OpenAIChatCompletionQuery::new(
                messages,
                response_schema.clone(),
            )
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
    pub(crate) id: String,
    pub(crate) object: String,
    pub(crate) created: i64,
    pub(crate) model: String,
    pub(crate) choices: Vec<Choice>,
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
    pub(crate) finish_reason: String,
    pub(crate) index: i64,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct Usage {
    prompt_tokens: i64,
    completion_tokens: i64,
    total_tokens: i64,
}
