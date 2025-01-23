pub(crate) async fn summarize_and_score_text_categorical(
    text: &str,
) -> anyhow::Result<ModelResponseSchema> {
    let query = crate::queries::openai::OpenAIChatCompletionQuery::new(
        crate::queries::openai::OpenAIChatCompletionQuery::system_prompt_and_content_to_messages(
            &crate::config::CONFIG.summarizer.categorical_system_prompt,
            text,
        ),
        schema_for_summarizer_response(),
    );

    let response = crate::CLIENT
        .post("https://api.openai.com/v1/chat/completions")
        .header(reqwest::header::USER_AGENT, "test")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", crate::config::CONFIG.title_scorer.api_key),
        )
        // .bearer_auth(&crate::config::CONFIG.api_key)
        .json(&query)
        .send()
        .await?;

    // If the request fails print the raw output for debugging.
    if let Err(e) = response.error_for_status_ref() {
        println!("Error: {}", e);
        println!("Raw output:\n{}", response.text().await?);
        return Err(anyhow::anyhow!("Error querying model: {}", e));
    }

    let model_response: crate::queries::openai::OpenAIChatCompletionResponse =
        response.json().await?;
    Ok(
        serde_json::from_str::<ModelResponseSchema>(&model_response.choices[0].message.content)
            .unwrap(),
    )
}

/// We enforce a json schema for the responses since we are working with structured data.
#[derive(Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ModelResponseSchema {
    #[schemars(required)]
    #[schemars(description = "Summary of the text")]
    summary: String,

    #[schemars(required)]
    #[schemars(description = "An array of tuples of the form (summary, ai_impact_score)")]
    ai_impact: crate::queries::title_categorical_scoring::Category,
}

/// Creates the json schema for the output following the OpenAI completely non-standard format...
fn schema_for_summarizer_response() -> crate::queries::openai::Schema {
    let mut schema = schemars::generate::SchemaSettings::default()
        .for_serialize()
        .with(|s| s.meta_schema = None)
        // The schema generator automatically adds "format" to the items specifying for example int64
        // or double. OpenAI does not support this.
        .with_transform(schemars::transform::RecursiveTransform(
            |schema: &mut schemars::Schema| {
                schema.remove("format");
            },
        ))
        .into_generator()
        .into_root_schema_for::<ModelResponseSchema>();

    // Remove title field from schema since OpenAI api does not support it.
    schema.as_object_mut().unwrap().remove("title");

    crate::queries::openai::Schema {
        name: "ai_relatedness_scores".to_string(),
        schema: serde_json::to_value(schema).unwrap(),
        strict: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_summarize_and_score_text_categorical() {
        let text = include_str!("../examples/Why-is-Big-Tech-hellbent-on-making-AI-opt-out?.txt");
        let res = summarize_and_score_text_categorical(text).await.unwrap();
        println!("Summary: {}", res.summary);
        println!("Score: {}", res.ai_impact);
    }

    #[tokio::test]
    async fn test_summarize_should_work_with_empty_text() {
        let res = summarize_and_score_text_categorical("").await.unwrap();
        println!("Summary: {}", res.summary);
        println!("Score: {}", res.ai_impact);
    }

    #[test]
    fn test_summarizer_schema() {
        let schema = schema_for_summarizer_response();
        println!("{:<10}", serde_json::to_string_pretty(&schema).unwrap());
    }
}
