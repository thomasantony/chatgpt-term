// Import the library from lib.rs
fn main() -> Result<(), serde_json::Error> {
    // Load chat log from chatlog.json file and deserialize it
    let chat_log: Vec<chatgpt_term::api::Message> =
        serde_json::from_str(&std::fs::read_to_string("chatlog.json").unwrap())?;

    println!("{:#?}", chat_log);
    Ok(())
}
