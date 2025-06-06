use serenity::all::Mentionable;
use crate::lib::{Context, Error};

#[poise::command(slash_command, prefix_command, rename = "ping")]
pub async fn ping_command(
    ctx: Context<'_>
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;

    let current_time = chrono::Utc::now();
    let user = ctx.author();

    let response: String = format!("Hello, {}. The current local time is <t:{}>", user.mention(), current_time.timestamp());
    ctx.reply(response).await?;

    Ok(())
}
