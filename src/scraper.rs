//! Simple scraper. Takes a link and simply returns the text message not loading any
//! dynamically fetched content.

pub(crate) async fn enrich_stories(
    stories: Vec<crate::Story>,
    export_text: bool,
) -> anyhow::Result<Vec<crate::Story>> {
    let mut scraped_stories = Vec::with_capacity(stories.len());

    let mut queries_set: tokio::task::JoinSet<anyhow::Result<crate::Story>> =
        tokio::task::JoinSet::new();

    for mut story in stories {
        queries_set.spawn(async move {
            let raw_text = crate::scraper::scrape_text(story.url.as_ref().ok_or(
                anyhow::anyhow!("URL not found. Title: {} Id: {}", story.title, story.id),
            )?)
            .await?;
            let trimmed_text = crate::scraper::html_to_trimmed_text(&raw_text)?;

            if export_text {
                use std::io::Write;
                let mut file =
                    std::fs::File::create(format!("tmp/{}.txt", story.title.replace(" ", "-")))?;
                file.write_all(trimmed_text.as_bytes())?;
            }

            story.text = Some(trimmed_text);

            Ok(story)
        });
    }

    while let Some(res) = queries_set.join_next().await {
        match res.expect("JoinSet to work") {
            Ok(text) => scraped_stories.push(text),
            Err(e) => tracing::error!(error =? e, "Error scraping story"),
        }
    }

    Ok(scraped_stories)
}

async fn scrape_text(url: &str) -> anyhow::Result<String> {
    let response = crate::CLIENT.get(url).send().await?.text().await?;
    tracing::info!(num_characters = response.len(), "Scraped {}", url);
    Ok(response)
}

fn html_to_trimmed_text(html: &str) -> anyhow::Result<String> {
    let text = html2text::config::plain()
        .raw_mode(true)
        .string_from_read(html.as_bytes(), 80)?;

    // Removes all lines containing the links.
    // [19]: /now
    // [20]: /uses
    let link_list_remover = regex::Regex::new(r"\[\d+\]:.+\n").unwrap();
    let text = link_list_remover.replace_all(&text, "");

    let link_number_re = regex::Regex::new(r"\[\d+\]").unwrap();
    let text = link_number_re.replace_all(&text, "");

    let bracket_re = regex::Regex::new(r"[\[|\]]").unwrap();
    let text = bracket_re.replace_all(&text, "");

    Ok(text.to_string())
}
