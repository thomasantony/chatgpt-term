use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatTermConfig {
    pub openai_api_key: String,
    pub openai_model: String,
    pub initial_prompt: String,
    pub max_tokens: u32,
}
// Implement default trait for Config with "gpt-3.5-turbo" as the default model
impl Default for ChatTermConfig {
    fn default() -> Self {
        Self {
            openai_api_key: String::from(""),
            openai_model: String::from("gpt-3.5-turbo"),
            initial_prompt: String::from(
                "You are Assistant, a very enthusiastic chatbot. You are chatting with a user.",
            ),
            max_tokens: 2000,
        }
    }
}
pub mod api;
pub mod app;
