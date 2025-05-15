use std::error::Error;
use rig::{completion::Prompt, pipeline::{self, Op}};
use serde_json::json;

use async_openai::{
    config::OpenAIConfig, types::{
        ChatCompletionFunctionsArgs, ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs
    }, Client
};

mod prompt_utils;
mod utils;
mod agents;
mod extractors;

use agents::ollama_agent;

async fn ask_with_history(
    client: &Client<OpenAIConfig>,
    message: &str,
    mut history: Vec<ChatCompletionRequestMessage>,
) -> Result<String, Box<dyn Error>> {
    history.push(ChatCompletionRequestMessage::User(
        ChatCompletionRequestUserMessageArgs::default()
            .content(message)
            .build()?
            .into()
    ));
    let request = CreateChatCompletionRequestArgs::default()
            .model("qwen3:14b")
            .temperature(0.7)
            .messages(history)
            .build()?;

    let response = client.chat().create(request).await?;
    return Ok(response.choices[0].message.content.clone().unwrap());
}

async fn function_calling(client: &Client<OpenAIConfig>, message: &str) -> Result<String, Box<dyn Error>> {
    let request = CreateChatCompletionRequestArgs::default()
        .model("qwen3:14b")
        .temperature(0.7)
        .messages(vec![
            ChatCompletionRequestMessage::System(
                ChatCompletionRequestSystemMessageArgs::default()
                .content("you are a helpful assistant")
                .build()?
                .into()
            ),
            ChatCompletionRequestMessage::User(
                ChatCompletionRequestUserMessageArgs::default()
                .content(message)
                .build()?
                .into()
            )
        ])
        .functions(vec![
            ChatCompletionFunctionsArgs::default()
            .name("recommend")
            .description("Provide a recommendation for any topic.")
            .parameters(json!({
                "type": "object",
                "properties": {
                    "topic": {
                        "type": "string",
                        "description": "The topic to recommend."
                    }
                },
                "required": ["topic"]
            }))
            .build()?
        ])
        .function_call("auto")
        .build()?;

    let response = client.chat().create(request).await?;
    return Ok(response.choices[0].message.content.clone().unwrap());
}

async fn ask_gpt(client: &Client<OpenAIConfig>, message: &str) -> Result<String, Box<dyn Error>> {
    let request = CreateChatCompletionRequestArgs::default()
        .model("qwen3:14b")
        .temperature(0.7)
        .messages(vec![
            ChatCompletionRequestMessage::System(
                ChatCompletionRequestSystemMessageArgs::default()
                .content("you are a helpful assistant")
                .build()?
                .into()
            ),
            ChatCompletionRequestMessage::User(
                ChatCompletionRequestUserMessageArgs::default()
                .content(message)
                .build()?
                .into()
            )
        ])
        .build()?;

    let response = client.chat().create(request).await?;
    return Ok(response.choices[0].message.content.clone().unwrap());
}

async fn prompt_engineering(client: &Client<OpenAIConfig>) -> Result<(), Box<dyn Error>> {
    let directory = "prompts";
    let text_files = utils::list_text_files_in_directory(directory);

    for (index, file) in text_files.iter().enumerate() {
        println!("{}: {}", index + 1, file);
    }

    loop {
        println!("Enter the index of the file you want to use or (0 to exit)");
        let mut file_index = String::new();
        std::io::stdin().read_line(&mut file_index).unwrap();
        let file_index: usize = file_index.trim().parse().unwrap();
        if file_index == 0 {
            break Ok(())
        } else if 1 <= file_index && file_index <= text_files.len() {
            let selected_file = &text_files[file_index - 1]; 
            let prompts = utils::load_and_parse_json_file(selected_file);
            for (index, prompt) in prompts.iter().enumerate() {
                println!("PROMPT: {}", index + 1);
                println!("{:?}", prompt);
                println!("REPLY");
                println!("{}", prompt_utils::prompt_llm(prompt.clone(), "qwen3:14b", client).await.unwrap());
            }
        } else {
            println!("Invalid file index");
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = OpenAIConfig::new()
        .with_api_key("test")
        .with_api_base("http://127.0.0.1:11434/v1");

    let client = Client::with_config(config);
    // let response = ask_gpt(&client, "What is the capital of France?").await?;
    
    let history = vec![
        ChatCompletionRequestMessage::System(
            ChatCompletionRequestSystemMessageArgs::default()
            .content("you are a helpful assistant")
            .build()?
            .into()
        ),
        ChatCompletionRequestMessage::User(
            ChatCompletionRequestUserMessageArgs::default()
            .content("What is the capital of France?")
            .build()?
            .into()
        ),
        ChatCompletionRequestMessage::Assistant(
            ChatCompletionRequestAssistantMessageArgs::default()
            .content("The capital of France is Paris.")
            .build()?
            .into()
        ),
    ];

    // let response = function_calling(&client, "Can you please recommend me a good time travel movie?").await?;

    // let response = ask_with_history(&client, "What is an intresting fact about this city?", history).await?;
    
    let d_response = extractors::extract_recommend_topic("qwen3:14b", "Can you please recommend me a very bad time travel movie?").await?;

    // prompt_engineering(&client).await?;
    // println!("Response: {}", response);
    //
    // let proxy_agent = ollama_agent::user_proxy_agent("qwen3:14b");
    // let response = proxy_agent.prompt("who are you?").await?;
    // let joke_researcher_agent = ollama_agent::joke_researcher_agent("qwen3:14b");
    // let joke_writer_agent = ollama_agent::joke_writer_agent("qwen3:14b");
    // let chain = pipeline::new()
    //     .prompt(joke_researcher_agent)
    //     .map(|response| response.unwrap())
    //     .map(|response| {
    //         format!(
    //             "
    //             Compose an insightful, humourous and socially aware joke on AI Engineer.
    //             Be sure to include the key elements that make it funny and
    //             relevant to the current social trends.

    //             Use the following information to write the joke:
    //             {}
                
    //             Expected Output: A concise and short one line joke on AI Engineer. 
    //             ",
    //             response,
    //         )
    //     })
    //     .prompt(joke_writer_agent);

    // let response = chain
    //     .call("
    //         Identify what makes the following topic: AI Engineer so funny.
    //         Be sure to include the key elements that make it humorous.
    //         Also, provide an analysis of the current social trends,
    //         and how it impacts the perception of humor.
            
    //         Expected Output: A comprehensive 3 paragraphs long report on the latest jokes.
    //     ")
    //     .await?;
    println!("{:?}", d_response);
    Ok(())
}
