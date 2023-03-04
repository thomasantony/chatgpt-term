// Import the library from lib.rs
use chatgpt_term::api::{ChatGPTClient, ChatLogEntry};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get the OpenAI API key from the environment
    let key = "OPENAI_API_KEY";
    let openai_api_key = env::var(key).unwrap_or_else(|_| panic!("{} not set", key));
    // Create a new client
    let client = ChatGPTClient::new(&openai_api_key, "gpt-3.5-turbo");
    chatgpt_term::app::run(client)?;

    Ok(())
}

// Mark as unused to avoid warnings
#[allow(dead_code)]
fn generate_dummy_data() -> std::io::Result<()> {
    // Generate some dummy ChatLogEntry objects
    let mut chat_log: Vec<ChatLogEntry> = Vec::new();
    for i in 0..10 {
        let entry = ChatLogEntry::new(&format!("Message {}", i), &format!("Response {}", i));
        chat_log.push(entry);
    }

    // Write the chat log to a file in prettified json format
    let chat_log_json = serde_json::to_string_pretty(&chat_log)?;
    std::fs::write("chatlog.json", chat_log_json)?;

    Ok(())
}
