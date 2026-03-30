mod agent;
mod tools;

use async_openai::{Client, config::OpenAIConfig};
use clap::Parser;
use std::{env, process};

use agent::Agent;

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    #[arg(short = 'p', long)]
    prompt: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let client = create_client();
    let mut agent = Agent::new(client, "deepseek-chat");

    agent.add_user_message(&args.prompt);

    let result = agent.run().await?;
    println!("{}", result);

    Ok(())
}

fn create_client() -> Client<OpenAIConfig> {
    let base_url =
        env::var("DEEPSEEK_BASE_URL").unwrap_or_else(|_| "https://api.deepseek.com".to_string());

    let api_key = env::var("DEEPSEEK_API_KEY").unwrap_or_else(|_| {
        eprintln!("DEEPSEEK_API_KEY is not set");
        process::exit(1);
    });


    let config = OpenAIConfig::new()
        .with_api_base(base_url)
        .with_api_key(api_key);

    Client::with_config(config)
}
