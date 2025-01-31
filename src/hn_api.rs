pub(crate) async fn get_hackernews_top_stories() -> anyhow::Result<Vec<crate::Story>> {
    let response = crate::CLIENT
        .get("https://hacker-news.firebaseio.com/v0/topstories.json")
        .send()
        .await?;
    let stories: Vec<i64> = response.json::<Vec<i64>>().await?
        [..crate::config::config().num_titles_to_request]
        .to_vec();

    // We can do this in parallel but this is good enough for now.
    let mut enriched_stories = Vec::with_capacity(stories.len());

    let mut queries_set: tokio::task::JoinSet<anyhow::Result<crate::Story>> =
        tokio::task::JoinSet::new();

    for story in stories {
        queries_set.spawn(async move {
            let response = crate::CLIENT
                .get(format!(
                    "https://hacker-news.firebaseio.com/v0/item/{}.json",
                    story
                ))
                .send()
                .await?;

            Ok(response.json::<crate::Story>().await?)
        });
    }

    while let Some(res) = queries_set.join_next().await {
        match res.expect("JoinSet to work") {
            Ok(story) => enriched_stories.push(story),
            Err(e) => tracing::error!(error =? e, "Error getting story"),
        }
    }

    Ok(enriched_stories)
}
