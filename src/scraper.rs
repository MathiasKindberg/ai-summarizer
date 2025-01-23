/// Simple scraper. Takes a link and simply returns the text message not loading any
/// dynamically fetched content.
pub(crate) async fn scrape_text(url: &str) -> anyhow::Result<String> {
    tracing::info!("Scraping {}", url);
    Ok(crate::CLIENT.get(url).send().await?.text().await?)
}

pub(crate) fn html_to_trimmed_text(html: &str) -> anyhow::Result<String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scrape_text() {
        let text = scrape_text(
            "https://christiantietze.de/posts/2025/01/using-2-editors-because-xcode-is-dumb/",
        )
        // let text = scrape_text("https://apnews.com/article/trump-ai-openai-oracle-softbank-son-altman-ellison-be261f8a8ee07a0623d4170397348c41")
        // let text = scrape_text(
        //     "https://diamondgeezer.blogspot.com/2025/01/londons-most-central-sheep.html",
        // )
        .await
        .unwrap();

        let text = html_to_trimmed_text(&text).unwrap();
        println!("{}", text);

        use std::io::Write;
        let mut file = std::fs::File::create("tmp/trimmed_scraped_text.txt").unwrap();
        file.write_all(text.as_bytes()).unwrap();
    }
}
