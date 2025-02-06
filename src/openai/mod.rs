pub(crate) mod summarizer;

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

impl std::fmt::Display for crate::openai::Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            crate::openai::Category::High => write!(f, "High"),
            crate::openai::Category::Medium => write!(f, "Medium"),
            crate::openai::Category::Low => write!(f, "Low"),
            crate::openai::Category::Zero => write!(f, "Zero"),
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct OpenAIChatCompletionQuery {
    model: String,
    messages: Vec<Message>,
    response_format: ResponseFormat,
}

impl OpenAIChatCompletionQuery {
    pub(crate) fn new(model: String, messages: Vec<Message>, schema: Schema) -> Self {
        Self {
            model,
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct Usage {
    pub(crate) prompt_tokens: i64,
    pub(crate) completion_tokens: i64,
    pub(crate) total_tokens: i64,
}
