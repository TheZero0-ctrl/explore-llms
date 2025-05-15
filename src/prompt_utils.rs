use std::error::Error;
use async_openai::{
    types::{ChatCompletionRequestMessage, CreateChatCompletionRequestArgs},
    config::OpenAIConfig,
    Client,
};

pub async fn prompt_llm(
    messages: Vec<ChatCompletionRequestMessage>,
    model: &str,
    client: &Client<OpenAIConfig>,
) -> Result<String, Box<dyn Error>> {
    let request = CreateChatCompletionRequestArgs::default()
            .model(model)
            .temperature(0.7)
            .messages(messages)
            .build()?;

    let response = client.chat().create(request).await?;
    return Ok(response.choices[0].message.content.clone().unwrap());
}
