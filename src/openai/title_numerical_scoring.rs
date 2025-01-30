//! Scores titles based on numbers like from 0 to 10.

pub(crate) async fn score_ai_impact(titles: Vec<String>) -> anyhow::Result<Vec<i64>> {
    let queries = crate::openai::convert_titles_to_messages(
        titles,
        crate::config::config().title_scorer.model.clone(),
        &crate::config::config().title_scorer.numerical_system_prompt,
        schema_for_model_response(),
    );

    let mut join_set = tokio::task::JoinSet::new();
    let mut model_responses: Vec<i64> = Vec::with_capacity(queries.len());

    for query in queries {
        join_set.spawn(async move {
            let response = crate::CLIENT
                .post("https://api.openai.com/v1/chat/completions")
                .header(reqwest::header::USER_AGENT, "test")
                .header(
                    reqwest::header::AUTHORIZATION,
                    format!("Bearer {}", crate::config::config().title_scorer.api_key),
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

            let model_response: crate::openai::OpenAIChatCompletionResponse =
                response.json().await?;
            Ok(serde_json::from_str::<ModelResponseSchema>(
                &model_response.choices[0].message.content,
            )
            .unwrap()
            .ai_relatedness_scores)
        });
    }

    while let Some(res) = join_set.join_next().await {
        match res.expect("Expecting join set to return ok") {
            Ok(scores) => model_responses.extend(scores),
            Err(err) => {
                tracing::error!(error =? err, "Error querying model, skipping");
            }
        }
    }

    Ok(model_responses)
}

/// Creates the json schema for the output following the OpenAI completely non-standard format...
fn schema_for_model_response() -> crate::openai::Schema {
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

    crate::openai::Schema {
        name: "ai_relatedness_scores".to_string(),
        schema: serde_json::to_value(schema).unwrap(),
        strict: true,
    }
}

/// We enforce a json schema for the responses since we are working with structured data.
#[derive(Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct ModelResponseSchema {
    #[schemars(required)]
    #[schemars(
        description = "An array of describing the ai relatedness of the titles from 0 to 10"
    )]
    ai_relatedness_scores: Vec<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_numerical_title_scoring() {
        let titles = vec![
            "GitHub introduces sub-issues, issue types and advanced search".to_string(),
            "TikTok goes dark in the US".to_string(),
            "The AMD Radeon Instinct MI300A's Giant Memory Subsystem".to_string(),
            "Show HN: LLMpeg".to_string(),
            "DeepSeek-R1".to_string(),
            "Metacognitive laziness: Effects of generative AI on learning motivation".to_string(),
        ];

        let res = score_ai_impact(titles.clone()).await.unwrap();
        titles.iter().zip(res.iter()).for_each(|(title, score)| {
            println!("{:>2}: {}", score, title);
        });
        assert_eq!(res.len(), titles.len());
    }

    #[test]
    fn test_numerical_schema() {
        let schema = schema_for_model_response();
        println!("{}", serde_json::to_string_pretty(&schema).unwrap());
    }
}
