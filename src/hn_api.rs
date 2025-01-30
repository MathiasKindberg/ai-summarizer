pub(crate) async fn get_hackernews_top_stories() -> Vec<crate::Story> {
    let response = crate::CLIENT
        .get("https://hacker-news.firebaseio.com/v0/topstories.json")
        .send()
        .await
        .unwrap();
    let stories: Vec<i64> = response.json::<Vec<i64>>().await.unwrap()
        [..crate::config::config().num_titles_to_request]
        .to_vec();

    // We can do this in parallel but this is good enough for now.
    let mut enriched_stories = Vec::with_capacity(stories.len());

    let mut queries_set = tokio::task::JoinSet::new();
    for story in stories {
        queries_set.spawn(async move {
            crate::CLIENT
                .get(format!(
                    "https://hacker-news.firebaseio.com/v0/item/{}.json",
                    story
                ))
                .send()
                .await
                .unwrap()
                .json::<crate::Story>()
                .await
                .unwrap()
        });
    }

    while let Some(res) = queries_set.join_next().await {
        enriched_stories.push(res.unwrap());
    }

    enriched_stories
}
