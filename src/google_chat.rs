// https://chat.googleapis.com/v1/spaces/AAAAkZT2Cu4/messages?key=AIzaSyDdI0hCZtE6vySjMm-WEfRq3CPzqKqqsHI&token=Gnrdf_mB45MtianvteUbzLojaXeUA6ozI3kgV5gKEXE

#[derive(Debug, serde::Serialize)]
struct Message {
    text: String,
}

fn story_to_message(story: &crate::Story) -> String {
    let crate::Story {
        title,
        url,
        summary,
        ai_impact_score,
        score,
        id,
        descendants: descendents,
        ..
    } = story;

    let url = url.as_ref().expect("url to be set");
    let descendents = descendents.as_ref().expect("descendents to be set");
    let ai_impact_score = ai_impact_score.as_ref().expect("ai impact score to be set");
    let summary = summary
        .as_ref()
        .expect("summary to be set")
        .iter()
        .map(|s| format!("{s}"))
        .collect::<Vec<String>>()
        .join("\n\n");

    format!(
        "*<{url}|{title}>*\nAI Impact: {ai_impact_score} | Votes: {score} | <https://news.ycombinator.com/item?id={id}|{descendents} Comments> \n\n{summary}\n\n"
    )
}

pub(crate) fn create_message(stories: Vec<crate::Story>) -> String {
    let mut message = String::new();
    message.push_str(&format!(
        "*Daily digest of top Hacker news AI stories as per {}*\n\n",
        crate::config::config().title_scorer.model
    ));

    for story in stories {
        message.push_str(&story_to_message(&story));
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
            &crate::config::config().google_chat_test_webhook_url,
        )
        .await
        .unwrap();
    }

    #[test]
    fn test_create_message() {
        let text = include_str!("examples/stories.json");
        let stories = serde_json::from_str::<Vec<crate::Story>>(text).unwrap();
        let message = create_message(stories);
        println!("{}", message);
        // assert!(!message.is_empty());
        // println!("{}", message);
    }
}
