pub async fn request(url: impl reqwest::IntoUrl) -> Result<reqwest::Response, reqwest::Error> {
    let client = reqwest::ClientBuilder::new()
        .user_agent("moth-fans-runtimes")
        .build()
        .expect("failed to build reqwest client");

    client.get(url).send().await
}
