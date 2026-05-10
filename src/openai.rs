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

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ReasoningEffort {
    Low,
    Medium,
    High,
}

impl std::str::FromStr for ReasoningEffort {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            other => Err(anyhow::anyhow!(
                "invalid OPENAI_REASONING_EFFORT {other:?}; expected low | medium | high"
            )),
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct OpenAIChatCompletionQuery {
    model: String,
    messages: Vec<Message>,
    response_format: ResponseFormat,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_effort: Option<ReasoningEffort>,
}

impl OpenAIChatCompletionQuery {
    pub(crate) fn new(
        model: String,
        messages: Vec<Message>,
        schema: Schema,
        reasoning_effort: Option<ReasoningEffort>,
    ) -> Self {
        Self {
            model,
            messages,
            response_format: ResponseFormat {
                response_type: "json_schema".to_string(),
                json_schema: schema,
            },
            reasoning_effort,
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
#[allow(unused)]
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
#[allow(unused)]
pub(crate) struct Choice {
    pub(crate) message: ResponseMessage,
    pub(crate) finish_reason: String,
    pub(crate) index: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct Usage {
    pub(crate) prompt_tokens: i64,
    pub(crate) completion_tokens: i64,
    pub(crate) total_tokens: i64,
}

pub(crate) async fn enrich_story(mut story: crate::Story) -> anyhow::Result<crate::Story> {
    let (summary, usage) = summarize_and_score_text_categorical(&story.title).await?;
    story.summary = Some(summary.summary);
    story.ai_impact_score = Some(summary.ai_impact);

    story.usage = Some(usage);
    Ok(story)
}

async fn summarize_and_score_text_categorical(
    text: &str,
) -> anyhow::Result<(SummaryResponse, crate::openai::Usage)> {
    use backon::Retryable;

    let query = crate::openai::OpenAIChatCompletionQuery::new(
        crate::config::config().model.clone(),
        crate::openai::OpenAIChatCompletionQuery::system_prompt_and_content_to_messages(
            &crate::config::config().system_prompt,
            text,
        ),
        schema_for_summarizer_response(),
        crate::config::config().reasoning_effort.clone(),
    );

    let model_response: crate::openai::OpenAIChatCompletionResponse = (|| async {
        let response = crate::CLIENT
            .post("https://api.openai.com/v1/chat/completions")
            .timeout(std::time::Duration::from_secs(180))
            .header(reqwest::header::USER_AGENT, "test")
            .bearer_auth(&crate::config::config().api_key)
            .json(&query)
            .send()
            .await?;

        if let Err(e) = response.error_for_status_ref() {
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("OpenAI request failed: {e}: {body}"));
        }

        Ok(response.json().await?)
    })
    .retry(crate::backoff::backoff_default())
    .sleep(tokio::time::sleep)
    .notify(|e: &anyhow::Error, duration: std::time::Duration| {
        tracing::warn!(
            error =? e,
            error_at =? duration.as_secs(),
            "Error querying OpenAI, retrying"
        );
    })
    .await?;

    let summary =
        serde_json::from_str::<SummaryResponse>(&model_response.choices[0].message.content)?;
    Ok((summary, model_response.usage))
}

/// We enforce a json schema for the responses since we are working with structured data.
#[derive(Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SummaryResponse {
    #[schemars(required)]
    #[schemars(description = "Summary of the text")]
    pub(crate) summary: Vec<String>,

    #[schemars(required)]
    #[schemars(description = "An array of tuples of the form (summary, ai_impact_score)")]
    pub(crate) ai_impact: crate::openai::Category,
}

/// Creates the json schema for the output following the OpenAI completely non-standard format...
fn schema_for_summarizer_response() -> crate::openai::Schema {
    let schema = schemars::generate::SchemaSettings::default()
        .with_transform(schemars::transform::RecursiveTransform(
            |schema: &mut schemars::Schema| {
                schema.remove("format");
            },
        ))
        .into_generator()
        .into_root_schema_for::<SummaryResponse>();

    crate::openai::Schema {
        name: "ai_relatedness_scores".to_string(),
        schema: serde_json::to_value(schema).expect("Failed to convert schema to json"),
        strict: true,
    }
}
