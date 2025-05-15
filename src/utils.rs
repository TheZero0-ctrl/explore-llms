use async_openai::types::ChatCompletionRequestMessage;

pub fn list_text_files_in_directory(directory_path: &str) -> Vec<String> {
    let mut file_paths = Vec::new();
    for entry in std::fs::read_dir(directory_path).unwrap() {
        let path = entry.unwrap().path();
        if path.is_file() {
            if let Some(extension) = path.extension() {
                if extension == "json" && !path.file_name().unwrap().to_string_lossy().starts_with("_") {
                    file_paths.push(path.to_string_lossy().to_string());
                }
            }
        }
    }
    file_paths
}

pub fn load_and_parse_json_file(file_path: &str) -> Vec<Vec<ChatCompletionRequestMessage>> {
    let content = std::fs::read_to_string(file_path).unwrap();
    let messages: Vec<Vec<ChatCompletionRequestMessage>> = serde_json::from_str(&content).unwrap();
    messages
}
