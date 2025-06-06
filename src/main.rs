mod commands;
mod lib;

use poise::{async_trait, serenity_prelude as serenity};
use crate::lib::{Data};

struct BotOnlineHandler;

#[async_trait]
impl serenity::EventHandler for BotOnlineHandler {
    async fn ready(&self, _: serenity::Context, ready: serenity::Ready) {
        println!("✅  {} is online!", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    
    let token = std::env::var("BOT_TOKEN").expect("BOT_TOKEN is not set.");

    let intents = serenity::GatewayIntents::non_privileged();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                commands::ping_command(),
                commands::schedule_command()
            ],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {})
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .event_handler(BotOnlineHandler)
        .await;
    
    client.unwrap().start().await.unwrap();
}
