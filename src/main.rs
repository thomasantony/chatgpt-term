// Import the library from lib.rs
use chatgpt_term::{
    api::{ChatGPTClient, ChatLogEntry},
    ChatTermConfig,
};
use gumdrop::Options;
use std::io::Write;

const MIN_MAX_TOKENS: u32 = 1000;
const MAX_MAX_TOKENS: u32 = 4096;

// Function to prompt user for a yes/no value until they enter a valid value
fn prompt_yes_no(prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut input = String::new();
    let mut stdout = std::io::stdout();
    let stdin = std::io::stdin();
    while input != "y" && input != "n" {
        print!("{}", prompt);
        stdout.flush()?;
        input.clear();
        stdin.read_line(&mut input)?;
        input = format!("{}", input.trim().to_lowercase());
    }
    Ok(input)
}

fn prompt_non_empty(prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut input = String::new();
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    while input.is_empty() {
        print!("{}", prompt);
        stdout.flush()?;
        input.clear();
        stdin.read_line(&mut input)?;
    }
    Ok(input.trim().to_string())
}

// Prompts for a valid integer with upper and lower bounds
fn prompt_valid_integer(prompt: &str, lo: u32, hi: u32) -> Result<u32, Box<dyn std::error::Error>> {
    let mut input = String::new();
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    loop {
        print!("{}", prompt);
        stdout.flush()?;
        input.clear();
        stdin.read_line(&mut input)?;
        if let Ok(val) = input.trim().parse::<u32>() {
            if val < lo || val > hi {
                println!("Value must be between {} and {}", lo, hi);
                continue;
            }
            return Ok(val);
        } else {
            println!("Invalid input");
            continue;
        }
    }
}

// Structure for holding command line arguments
#[derive(Debug, Options)]
struct Args {
    #[options(help = "print help message")]
    help: bool,
    #[options(help = "session file to load")]
    session: Option<String>,
    #[options(help = "reconfigure the application")]
    reconfigure: bool,
}

fn configure() -> Result<ChatTermConfig, Box<dyn std::error::Error>> {
    // Prompt the user to get the OpenAI API key and save it to the config file
    let api_key = prompt_non_empty("Enter OpenAI API Key: ")?;
    let mut config = ChatTermConfig::default();
    config.openai_api_key = api_key;

    // Display current initial prompt and ask user if they want to change it
    println!("Initial prompt:\n\n{}\n", config.initial_prompt);

    let change_prompt = prompt_yes_no("Change initial prompt? (y/n): ")?;
    if change_prompt == "y" {
        // Prompt the user to get the new initial prompt from stdin (without using rpassword)
        config.initial_prompt = prompt_non_empty("Enter new initial prompt:")?;
    }

    // Prompt for max tokens
    config.max_tokens = prompt_valid_integer("Enter max tokens: ", MIN_MAX_TOKENS, MAX_MAX_TOKENS)?;

    Ok(config)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let args = Args::parse_args_default_or_exit();

    // Use confy to load config file into struct
    let config: ChatTermConfig = confy::load("chatgpt-term", None).unwrap_or_default();

    // If the this is the first time or if the user wants to configure the application, run the configuration function
    let config = if config.openai_api_key.is_empty() || args.reconfigure {
        let config = configure()?;
        println!("Saving config ...");
        confy::store("chatgpt-term", None, &config)?;
        config
    } else {
        config
    };

    // Create a new client using config
    let client = ChatGPTClient::new(config);
    chatgpt_term::app::run(client, args.session)?;

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
