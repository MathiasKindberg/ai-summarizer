#[derive(Debug, serde::Serialize)]
struct Message {
    text: String,
}

fn story_to_message(story: &crate::Story) -> anyhow::Result<String> {
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

    let url = url.as_ref().ok_or(anyhow::anyhow!("url to be set"))?;
    let descendents = descendents
        .as_ref()
        .ok_or(anyhow::anyhow!("descendents to be set"))?;
    let ai_impact_score = ai_impact_score
        .as_ref()
        .ok_or(anyhow::anyhow!("ai impact score to be set"))?;
    let summary = summary
        .as_ref()
        .ok_or(anyhow::anyhow!("summary to be set"))?
        .to_vec()
        .join("\n\n");

    Ok(format!(
        "*<{url}|{title}>*\nAI Impact: {ai_impact_score} | Votes: {score} | <https://news.ycombinator.com/item?id={id}|{descendents} Comments>\n\n{summary}\n\n"
    ))
}

pub(crate) fn create_message(stories: Vec<crate::Story>) -> anyhow::Result<String> {
    let mut message = String::new();
    message.push_str(&format!(
        "*Daily digest of top Hacker news AI stories as per {}*\n\n",
        crate::config::config().summarizer.model
    ));

    for story in stories {
        message.push_str(&story_to_message(&story)?);
    }

    const GITHUB_REPO_URL: &str = "https://github.com/mathiaskindberg/ai-summarizer";
    message.push_str(&format!("<{GITHUB_REPO_URL}|Source code>"));

    Ok(message)
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
