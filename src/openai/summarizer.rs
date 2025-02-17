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
    let query = crate::openai::OpenAIChatCompletionQuery::new(
        crate::config::config().summarizer.model.clone(),
        crate::openai::OpenAIChatCompletionQuery::system_prompt_and_content_to_messages(
            &crate::config::config().summarizer.system_prompt,
            text,
        ),
        schema_for_summarizer_response(),
    );

    let response = crate::CLIENT
        .post("https://api.openai.com/v1/chat/completions")
        .header(reqwest::header::USER_AGENT, "test")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", crate::config::config().summarizer.api_key),
        )
        // .bearer_auth(&crate::config::config().api_key)
        .json(&query)
        .send()
        .await?;

    // If the request fails print the raw output for debugging.
    if let Err(e) = response.error_for_status_ref() {
        println!("Error: {}", e);
        println!("Raw output:\n{}", response.text().await?);
        return Err(anyhow::anyhow!("Error querying model: {}", e));
    }

    let model_response: crate::openai::OpenAIChatCompletionResponse = response.json().await?;
    let summary =
        serde_json::from_str::<SummaryResponse>(&model_response.choices[0].message.content)
            .unwrap();
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
        .into_root_schema_for::<SummaryResponse>();

    // Remove title field from schema since OpenAI api does not support it.
    schema.as_object_mut().unwrap().remove("title");

    crate::openai::Schema {
        name: "ai_relatedness_scores".to_string(),
        schema: serde_json::to_value(schema).unwrap(),
        strict: true,
    }
}
