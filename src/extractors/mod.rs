use std::error::Error;
pub use serde::{Deserialize, Serialize};
pub use schemars::JsonSchema;

use rig::providers;

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub enum Rating {
    Good,
    Bad,
    Terrible
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct Recommend {
    pub topic: String,
    pub rating: Option<Rating>
}
pub async fn extract_recommend_topic(model: &str, message: &str) -> Result<Recommend, Box<dyn Error>> {
    let client = providers::ollama::Client::new();
    let extractor = client
        .extractor::<Recommend>(model)
        .preamble(r#"
            You are a topic and rating extractor. Identify the topic and rating of the message.
            identifying the topics is compulsory but rating is optional.

            for rating, use one of the following:
            - Good
            - Bad
            - Terrible

            **Output Format:**
            {
                "topic": "topic",
                "rating": "Good" | "Bad" | "Terrible"
            }

            Ensure that rating values are exactly "Good", "Bad" or "Terrible".
            Do not use lowercase or different variations.
        "#)
        .build();

    Ok(extractor.extract(message).await?)
}
