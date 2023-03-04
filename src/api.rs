use core::str;
use std::collections::VecDeque;

use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};

// Struct holds information from a chatgpt session including prior messages and responses
pub struct ChatGPTSession {
    // chat log is a vector of tuples of the form (message, response, num_tokens_message, num_tokens_response)
    pub chatlog: Vec<ChatLogEntry>,
    pub max_tokens: u32,
    pub client: ChatGPTClient,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChatLogEntry {
    pub message: String,
    pub response: String,
    pub num_tokens_message: u32,
    pub num_tokens_response: u32,
}
impl ChatLogEntry {
    pub fn new(message: &str, response: &str) -> Self {
        Self {
            message: String::from(message),
            response: String::from(response),
            num_tokens_message: 0,
            num_tokens_response: 0,
        }
    }
}
impl ChatGPTSession {
    // Initialize a new ChatGPTSession with a ChatGPTClient and max_tokens
    pub fn new(client: ChatGPTClient, max_tokens: u32) -> Self {
        Self {
            chatlog: Vec::new(),
            max_tokens,
            client,
        }
    }
    // Add data freom log file
    pub fn with_log_file(mut self, path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let entries: Vec<ChatLogEntry> = serde_json::from_str(&std::fs::read_to_string(path)?)?;
        self.chatlog = entries;
        Ok(self)
    }

    // Reset the chatlog
    pub fn reset(&mut self) {
        self.chatlog = Vec::new();
    }

    // Send a message to the ChatGPT API
    pub fn send_message(
        &mut self,
        message: &str,
    ) -> Result<ChatLogEntry, Box<dyn std::error::Error>> {
        // Add previous response and then the message before that and so on as long as the total number of tokens
        // is less than max_tokens
        let mut messages: VecDeque<Message> = VecDeque::new();

        let message = Message::new(message, "user");
        let mut num_tokens = message.content.split(' ').count() as u32;

        for entry in self.chatlog.iter().rev() {
            // First add the last response
            let resp_tokens = entry.num_tokens_response;
            if resp_tokens + num_tokens > self.max_tokens {
                break;
            }
            messages.push_front(Message::new(&entry.response, "assistant"));
            num_tokens += resp_tokens;

            // Then add the message that generated the response
            let message_tokens = entry.num_tokens_message;

            if message_tokens + num_tokens > self.max_tokens {
                break;
            }
            messages.push_front(Message::new(&entry.message, "user"));
            num_tokens += message_tokens;
        }
        messages.push_back(message);

        // Make API request to get ChatLogEntry
        let response = self.client.send_request(messages.into_iter())?;

        // // Create a fake ChatLogEntry with a dummy response
        // let response = ChatLogEntry::new(&message.content, "Some response from bot");
        self.chatlog.push(response.clone());
        Ok(response)
    }
}

// Struct representing a ChatGPT client with an auth token
// Uses a type state marker to represent the state of the client
pub struct ChatGPTClient {
    // ChatGPT auth token
    pub auth_token: String,
    // reqwest client
    pub client: Client,
    // model name
    pub model: String,
}

// A type representing a ChatGPT Message
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub content: String,
    pub role: String,
}

impl Message {
    pub fn new(content: &str, role: &str) -> Self {
        Self {
            content: String::from(content),
            role: String::from(role),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct ChatGPTRequest {
    #[serde(rename = "model")]
    model: String,
    #[serde(rename = "messages")]
    messages: Vec<Message>,
}

impl ChatGPTClient {
    // Construct new client from auth token, initializes reqwest client
    pub fn new(auth_token: &str, model: &str) -> Self {
        Self {
            auth_token: String::from(auth_token),
            client: Client::new(),
            model: String::from(model),
        }
    }
    // Create new session consuming the client
    // FIXME: Change this later to use a reference to a client
    pub fn new_session(self, max_tokens: u32) -> ChatGPTSession {
        ChatGPTSession::new(self, max_tokens)
    }
    // Send a request to the ChatGPT API
    // Example API request payload:
    // {"model":"gpt-3.5-turbo","messages":[{"content":"Hello, this is a test","role":"user"}]}
    pub fn send_request(
        &self,
        messages: impl Iterator<Item = Message>,
    ) -> Result<ChatLogEntry, Box<dyn std::error::Error>> {
        let request: ChatGPTRequest = ChatGPTRequest {
            model: self.model.clone(),
            messages: messages.collect(),
        };

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            format!("Bearer {}", self.auth_token).parse().unwrap(),
        );

        headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
        let json_data = serde_json::to_string(&request).unwrap();
        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions".to_string())
            .headers(headers)
            .body(json_data)
            .send()
            .unwrap()
            .json::<serde_json::Value>()
            .unwrap();

        // if the response is an error, cast it into an error and return Err()
        if response["error"].is_object() {
            let error = response["error"]["message"].as_str().unwrap();
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                error,
            )));
        }
        // Create the ChatLogEntry from the response
        let prompt_tokens = response["usage"]["prompt_tokens"].as_i64().unwrap();
        let answer_tokens = response["usage"]["completion_tokens"].as_i64().unwrap();
        let answer = response["choices"][0]["message"]["content"]
            .as_str()
            .unwrap();
        let answer = Message::new(answer, "assistant");
        let prompt = Message::new(
            &request.messages[request.messages.len() - 1].content,
            "user",
        );
        let entry = ChatLogEntry {
            message: prompt.content,
            response: answer.content,
            num_tokens_message: prompt_tokens as u32,
            num_tokens_response: answer_tokens as u32,
        };
        Ok(entry)
    }
}
