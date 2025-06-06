use crate::lib::{Context, Error};
use poise::{serenity_prelude as serenity};
use serenity::all::{CreateMessage, Mentionable};

async fn schedule_message(ctx: serenity::Context, user: serenity::User, time: std::time::Duration, message: String) {
    tokio::time::sleep(time).await;
    
    let builder = CreateMessage::new().content(message);

    if let Err(e) = user.direct_message(&ctx, builder).await {
        eprintln!("❌ Failed to send message to {}: {:?}", user.name, e);
        return;
    }
    
    println!("Message to {} was successfully scheduled!", user.name);
}

#[poise::command(slash_command, prefix_command, rename = "schedule")]
pub async fn schedule_command(
    ctx: Context<'_>,
    #[description = "Member to schedule message to."] member: serenity::Member,
    #[description = "Length before message is sent to them. (1s, 1m, 1d, 1w, 1M, 1Y)"] time: String,
    #[description = "The message you want to send."] message: String,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;

    let parsed_duration = parse_duration::parse(&time);

    if parsed_duration.is_err() {
        ctx.reply(format!("{} is not a valid time format.", time)).await?;
        return Err(Error::from("Invalid time format."));
    }

    let time = parsed_duration.unwrap();
    let new_message = format!("**{}**: {}", ctx.author().name, message);
    
    tokio::spawn(schedule_message(ctx.serenity_context().clone(), member.user.clone(), time, new_message));
    let response: String = format!("Message \"{}\" is scheduled to deliver to {} in {} seconds.", message, member.mention(), time.as_secs());
    
    ctx.reply(response).await?;

    Ok(())
}
