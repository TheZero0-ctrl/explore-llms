use rig::{agent::Agent, completion::CompletionModel, providers};

pub fn user_proxy_agent(model: &str) -> Agent<impl CompletionModel> {
    let client = providers::ollama::Client::new();
    client
        .agent(model)
        .preamble("You are proxy agent for user. You instruct and review other agents.")
        .build()
}

pub fn python_developer_agent(model: &str) -> Agent<impl CompletionModel> {
    let client = providers::ollama::Client::new();
    client
        .agent(model)
        .preamble("You are python developer. You write python code.")
        .build()
}

pub fn joke_researcher_agent(model: &str) -> Agent<impl CompletionModel> {
    let client = providers::ollama::Client::new();
    client
        .agent(model)
        .preamble(r#"
            Driven by slapsticDriven by slapstick humor, you are a seasoned joke researcher
            who knows what makes people laugh. You jhave a knack for finding
            the funny in everyday situations and can turn a dull moment into a laugh riot."#)
        .additional_params(serde_json::json!({"role": "Senior Joke Researcher", "goal": "Research what makes things funny about the following {topic}"}))
        .build()
}

pub fn joke_writer_agent(model: &str) -> Agent<impl CompletionModel> {
    let client = providers::ollama::Client::new();
    client
        .agent(model)
        .preamble(r#"
            You are a joke writer with a flair for humor. You can turn a 
            simple idea into a laugh riot. You have a way with words and
            can make people laugh with just a few lines."#)
        .build()
}
