use crate::lib::{Context, Error};
use glob::glob;
use poise::serenity_prelude as serenity;
use serde::{Deserialize, Serialize};
use serenity::all::{Attachment, CreateAttachment, CreateMessage, Mentionable};
use std::io::Write;
use std::path::PathBuf;

const DIRECTORY: &str = "./data";
const MESSAGES_CSV: &str = "./data/scheduled_messages.csv";
const ATTACHMENT_DIRECTORY: &str = "./data/attachments";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScheduledMessage {
    pub uuid: String,
    pub user_id: u64,
    pub message: String,
    pub send_at: i64, // Unix timestamp
}

fn remove_message_from_file(uuid: &str) {
    if let Ok(file) = std::fs::File::open(MESSAGES_CSV) {
        let reader = std::io::BufReader::new(file);
        let mut rdr = csv::ReaderBuilder::new().has_headers(false).from_reader(reader);
        let entries: Vec<ScheduledMessage> = rdr.deserialize().filter_map(Result::ok).collect();

        let remaining: Vec<_> = entries.into_iter().filter(|entry| entry.uuid != uuid).collect();

        let file = std::fs::File::create(MESSAGES_CSV).expect("Could not recreate file");
        let writer = std::io::BufWriter::new(file);
        let mut wtr = csv::WriterBuilder::new().has_headers(false).from_writer(writer);
        for msg in remaining {
            let _ = wtr.serialize(msg);
        }
        let _ = wtr.flush();
    }

    let pattern = format!("{}/{}*", ATTACHMENT_DIRECTORY, uuid);
    let matched_files: Vec<PathBuf> = glob(&pattern)
        .expect("Failed to read glob pattern")
        .filter_map(Result::ok)
        .collect();

    for file in matched_files {
        match std::fs::remove_file(&file) {
            Ok(_) => println!("🗑️ Deleted file: {}", file.display()),
            Err(e) => eprintln!("❌ Failed to delete file {}: {}", file.display(), e),
        }
    }
}

async fn add_schedule_to_file(
    user: &serenity::User,
    message: &String,
    time: std::time::Duration,
    additional_file: Option<Attachment>
) -> Result<String, Error> {
    std::fs::create_dir_all(DIRECTORY)?;  // Make sure directory exists.
    std::fs::create_dir_all(ATTACHMENT_DIRECTORY)?;  // Make sure directory exists.

    let id = uuid::Uuid::new_v4().to_string();

    if additional_file.is_some() {
        let unwrapped_file = additional_file.unwrap();
        let raw_file_name = &unwrapped_file.filename;
        let file_name: String;

        if raw_file_name.split('.').collect::<Vec<&str>>().len() > 1 {
            file_name = format!("{}.{}", id, raw_file_name.split(".").last().unwrap());
        } else {
            file_name = format!("{}.{}", id, "txt");
        }

        let file_path = format!("{}/{}", ATTACHMENT_DIRECTORY, file_name);
        let file_bytes = unwrapped_file.download().await?;

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(file_path)?;

        file.write_all(&file_bytes)?;
    }

    let scheduled = ScheduledMessage {
        uuid: id.clone(),
        user_id: user.id.get(),
        message: message.clone(),
        send_at: (chrono::Utc::now() + chrono::Duration::from_std(time).unwrap()).timestamp(),
    };

    let mut wtr = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(MESSAGES_CSV)?);

    wtr.serialize(&scheduled)?;
    wtr.flush()?;

    Ok(id)
}

pub async fn reschedule_messages(ctx: serenity::Context) -> Result<(), Error> {
    let file = match std::fs::File::open(MESSAGES_CSV) {
        Ok(f) => f,
        Err(_) => {
            println!("No scheduled messages file found at {}, nothing to reschedule.", MESSAGES_CSV);
            return Ok(()); // No file? Nothing to reschedule.
        }
    };

    println!("🔃 Reading scheduled messages from {}", MESSAGES_CSV);

    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(file);

    for result in rdr.deserialize::<ScheduledMessage>() {
        match result {
            Ok(msg) => {
                println!("⏳ Deserialized scheduled message: {:?}", msg);
                let now = chrono::Utc::now().timestamp();
                let delay_secs = (msg.send_at - now).max(0) as u64;
                let delay = std::time::Duration::from_secs(delay_secs);

                let user_id = serenity::UserId::new(msg.user_id);
                let ctx_clone = ctx.clone();
                let uuid = msg.uuid.clone();
                let message = msg.message.clone();

                tokio::spawn(async move {
                    if let Ok(user) = user_id.to_user(&ctx_clone).await {
                        println!(
                            "⏳ Rescheduling message to {} (UUID: {}) in {} seconds.",
                            user.name, uuid, delay_secs
                        );
                        schedule_message(ctx_clone, user, delay, message, uuid).await;
                    } else {
                        eprintln!("❌ Failed to fetch user with ID {}", msg.user_id);
                    }
                });
            }
            Err(e) => {
                eprintln!("❌ Failed to deserialize a scheduled message: {:?}", e);
            }
        }
    }

    Ok(())
}

async fn schedule_message(
    ctx: serenity::Context,
    user: serenity::User,
    time: std::time::Duration,
    message: String,
    uuid: String,
) {
    tokio::time::sleep(time).await;

    let pattern = format!("{}/{}*", ATTACHMENT_DIRECTORY, uuid);
    let potential_file_paths: Vec<PathBuf> = glob(&pattern)
        .expect("Failed to read glob pattern")
        .filter_map(Result::ok)
        .collect();

    for path in &potential_file_paths {
        println!("Resolved Path: {}", path.display());
    }
    
    let mut builder = CreateMessage::new().content(message.clone());

    if let Some(first_path) = potential_file_paths.first() {
        println!("Attaching File: {}", first_path.display());

        match CreateAttachment::path(first_path).await {
            Ok(attachment) => {
                builder = builder.add_file(attachment);
            }
            Err(e) => {
                eprintln!("❌ Failed to attach file: {}", e);
            }
        }
    }

    if let Err(e) = user.direct_message(&ctx, builder).await {
        eprintln!("❌ Failed to send message to {}: {:?}", user.name, e);
        return;
    }

    println!("✅ Message to {} was successfully sent!", user.name);
    remove_message_from_file(&uuid);
}

#[poise::command(slash_command, prefix_command, rename = "schedule")]
pub async fn schedule_command(
    ctx: Context<'_>,
    #[description = "Member to schedule message to."] member: serenity::Member,
    #[description = "Length before message is sent to them. (1s, 1m, 1d, 1w, 1M, 1Y)"] time: String,
    #[description = "The message you want to send."] message: String,
    #[description = "An additional attachment."] additional_file: Option<Attachment>,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;

    let parsed_duration = parse_duration::parse(&time);

    if parsed_duration.is_err() {
        ctx.reply(format!("{} is not a valid time format.", time)).await?;
        return Err(Error::from("Invalid time format."));
    }

    let time = parsed_duration.unwrap();
    let new_message = format!("**{}**: {}", ctx.author().name, message);

    if additional_file.clone().unwrap().size >= 25 * 1000000 {
        let response: String = format!("Your attachment is too large (25 MB Maximum): {} MB", additional_file.clone().unwrap().size / 1000000);

        ctx.reply(response).await?;
        return Ok(())
    }

    match add_schedule_to_file(&member.user, &new_message, time, additional_file).await {
        Ok(uuid) => {
            println!("✅  Message saved with UUID {}", uuid);
            tokio::spawn(schedule_message(
                ctx.serenity_context().clone(),
                member.user.clone(),
                time,
                new_message.clone(),
                uuid,
            ));
        }
        Err(e) => {
            eprintln!("❌ Failed to save message: {:?}", e);
        }
    }
    let response: String = format!("Message \"{}\" is scheduled to deliver to {} in {} seconds.", message, member.mention(), time.as_secs());
    
    ctx.reply(response).await?;

    Ok(())
}
