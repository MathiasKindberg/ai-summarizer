// https://chat.googleapis.com/v1/spaces/AAAAkZT2Cu4/messages?key=AIzaSyDdI0hCZtE6vySjMm-WEfRq3CPzqKqqsHI&token=Gnrdf_mB45MtianvteUbzLojaXeUA6ozI3kgV5gKEXE

#[derive(Debug, serde::Serialize)]
struct Message {
    text: String,
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
}
