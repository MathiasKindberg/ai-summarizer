// https://chat.googleapis.com/v1/spaces/AAAAkZT2Cu4/messages?key=AIzaSyDdI0hCZtE6vySjMm-WEfRq3CPzqKqqsHI&token=Gnrdf_mB45MtianvteUbzLojaXeUA6ozI3kgV5gKEXE

#[derive(Debug, serde::Serialize)]
struct Message {
    text: String,
}

pub(crate) fn create_message(summaries: Vec<crate::openai::summarizer::SummaryResponse>) -> String {
    let mut message = String::new();

    for summary in summaries {
        message.push_str(&format!("{:#?}\n", summary));
    }

    message
}

pub(crate) async fn send_message(message: String, url: &str) -> anyhow::Result<()> {
    let res = crate::CLIENT
        .post(url)
        .json(&Message { text: message })
        .send()
        .await?;

    res.error_for_status()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_send_message() {
        send_message(
            "Hello, world!".to_string(),
            &crate::config::CONFIG.google_chat_test_webhook_url,
        )
        .await
        .unwrap();
    }

    #[test]
    fn test_create_message() {
        let summaries = vec![
            crate::openai::summarizer::SummaryResponse {
                summary: vec![
                   "The document from the Dicastery for the Doctrine of the Faith discusses the ethical and anthropological implications of artificial intelligence (AI) from a Christian perspective, highlighting the distinction between human intelligence and AI, along with situations in which AI can promote or hinder human dignity.".to_string(),
                    "It emphasizes that AI should serve humanity and the common good, urging a responsible and moral approach to the development and application of technology while addressing potential risks of misinformation, privacy concerns, and the role of AI in various sectors like education, healthcare, and warfare.".to_string(),
                ],ai_impact: crate::openai::Category::High,
            },
            crate::openai::summarizer::SummaryResponse {
                summary: vec![
                    "The JavaScript Temporal object is set to modernize date and time handling in JavaScript, replacing the longstanding, problematic Date object with a robust API including support for time zones and various calendar systems.".to_string(),
                    "Experimental implementations of Temporal are beginning to roll out in browsers, promising significant improvements for web developers who require precise date and time functionalities.".to_string(),
                ],
                ai_impact: crate::openai::Category::High,
            },
            crate::openai::summarizer::SummaryResponse {
                summary: vec![
                    "Mistral AI has introduced Mistral Small 3, a latency-optimized 24B-parameter model under Apache 2.0, which performs competitively against larger models and is optimized for local deployment and fast response times.".to_string(),
                    "The model is specifically tailored for various generative AI applications, allowing for customization and fine-tuning for specific industries, further enhancing its utility in open-source development.".to_string(),
                ],
                ai_impact: crate::openai::Category::High,
            },
            crate::openai::summarizer::SummaryResponse {
                summary: vec![
                    "The author shares the journey of building a homemade pipe organ from scratch, detailing the design, construction process, and challenges encountered along the way.".to_string(),
                    "The narrative emphasizes DIY experimentation in music creation without a professional background, showcasing the joy and learning involved in crafting musical instruments.".to_string(),
                ],
                ai_impact: crate::openai::Category::Low,
            },
            crate::openai::summarizer::SummaryResponse {
                summary: vec![
                    "The article critiques Rust's `core::io::BorrowedBuf`, emphasizing its limitations and the problems it introduces, particularly regarding ease of use and integration with existing code.".to_string(),
                    "It discusses alternatives like `MaybeUninit` for handling uninitialized memory, but ultimately concludes that no ideal solution currently exists, which complicates performance optimizations in Rust.".to_string(),
                ],
                ai_impact: crate::openai::Category::Low,
            },
        ];

        let message = create_message(summaries);
        assert!(!message.is_empty());
        println!("{}", message);
    }
}
