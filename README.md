# A terminal-based ChatGPT client

This is a completely terminal based ChatGPT client that uses OpenAI's [ChatGPT API](https://platform.openai.com/docs/guides/chat). You can provide your own API key and recreate the ChatGPT experience in the terminal.

## Usage

```
$ ./target/debug/chatgpt-term --help
Usage: ./target/debug/chatgpt-term [OPTIONS]

Optional arguments:
-h, --help print help message
-s, --session SESSION session file to load
-r, --reconfigure reconfigure the application
```
Simply start the program as `chatgpt-term`. On the first run, it will prompt you to enter the API key and initial prompt. You can use the mouse/trackpad to scroll the chat log.

This is a simple proof-of-concept and may be expanded upon as time permits. I do welcome pull requests with improvements.

## Change application config

This prompts for the configuration settings at startup again.

`chatgpt-term --reconfigure`

## Continue an existing session

This can use a preexisting session file to continue a previous conversation.

`chatgpt-term --session <session-file.json>`
