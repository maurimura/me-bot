use async_trait::async_trait;
use serenity::client::{Client, Context, EventHandler};
use serenity::framework::standard::{
    macros::{command, group},
    CommandResult, StandardFramework,
};
use serenity::model::channel::{ChannelType, Message, Reaction};
use serenity::model::id::GuildId;
use serenity::model::prelude::Ready;

use std::env;

use dotenv::dotenv;

#[group]
#[commands(ping, echo, shop)]
struct General;

const SHOP_CHANNEL: &str = "🛒-shop";
const STOCK_CHANNEL: &str = "📦-stock";
struct Handler;

impl Handler {
    async fn reply(&self, original_message: Message, reply_message: Message) {
        println!("At reply: {}", original_message.content);
        println!("At reply: {}", reply_message.content);
    }
}

#[async_trait]
trait Shop {
    async fn add(&self, item: Message);
    async fn bought(&self, ctx: Context, item: Reaction);
}

#[async_trait]
impl Shop for Handler {
    async fn add(&self, message: Message) {
        println!("{:?}", message)
    }

    async fn bought(&self, ctx: Context, reaction: Reaction) {
        if reaction.emoji.unicode_eq("✅") {
            let guild = reaction
                .guild_id
                .unwrap()
                .to_guild_cached(ctx.clone())
                .await
                .unwrap();

            let stock_channel = guild
                .channel_id_from_name(ctx.clone(), STOCK_CHANNEL)
                .await
                .unwrap();

            let channel_id = reaction.channel_id;
            let message_iter = channel_id
                .messages(ctx.clone(), |retriver| {
                    retriver.around(reaction.message_id).limit(1)
                })
                .await
                .unwrap();

            for mut message in message_iter {
                let content = message.content.clone();
                message
                    .edit(ctx.clone(), |message| {
                        message.content(format!("~~{}~~", content.clone()))
                    })
                    .await
                    .unwrap();

                stock_channel.say(ctx.clone(), content).await.unwrap();
            }
        }
        // println!("NOT ADD: {:?}", reaction)
    }
}

#[async_trait]
trait Stock {
    async fn consumed(&self, ctx: Context, item: Reaction);
    async fn buy(&self, item: Reaction);
}

#[async_trait]
impl Stock for Handler {
    async fn buy(&self, message: Reaction) {
        println!("{:?}", message)
    }
    async fn consumed(&self, ctx: Context, reaction: Reaction) {
        let guild = reaction
            .guild_id
            .unwrap()
            .to_guild_cached(ctx.clone())
            .await
            .unwrap();

        let channel_id = reaction.channel_id;
        let message_iter = channel_id
            .messages(ctx.clone(), |retriver| {
                retriver.around(reaction.message_id).limit(1)
            })
            .await
            .unwrap();

        for mut message in message_iter {
            let current_content = message.content.clone();

            if reaction.emoji.unicode_eq("✅") {
                message
                    .edit(ctx.clone(), |message| {
                        message.content(format!("~~{}~~", current_content))
                    })
                    .await
                    .unwrap()
            }

            if reaction.emoji.unicode_eq("♻️") {
                message
                    .edit(ctx.clone(), |message| {
                        message.content(format!("~~{}~~", current_content.clone()))
                    })
                    .await
                    .unwrap();

                let shop_channel = guild
                    .channel_id_from_name(ctx.clone(), SHOP_CHANNEL)
                    .await
                    .unwrap();

                shop_channel
                    .say(ctx.clone(), current_content)
                    .await
                    .unwrap();
            }
        }
    }
}
#[async_trait]
impl EventHandler for Handler {
    async fn reaction_add(&self, ctx: Context, added_reaction: Reaction) {
        let channel = added_reaction
            .channel_id
            .to_channel_cached(ctx.clone())
            .await
            .unwrap();

        let guild_channel = channel.guild().unwrap();
        let name = guild_channel.name.as_str();
        match name {
            SHOP_CHANNEL => self.bought(ctx, added_reaction).await,
            STOCK_CHANNEL => self.consumed(ctx, added_reaction).await,
            _ => println!("{}", name),
        }
    }

    async fn ready(&self, _ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }

    async fn cache_ready(&self, ctx: Context, guilds: Vec<GuildId>) {
        if guilds.len() > 0 {
            let guild_id = guilds[0];
            let guild = guild_id.to_guild_cached(ctx.clone()).await.unwrap();
            match guild.channel_id_from_name(ctx.clone(), SHOP_CHANNEL).await {
                Some(_) => {}
                None => {
                    guild
                        .create_channel(ctx.clone(), |c| {
                            c.name(SHOP_CHANNEL).kind(ChannelType::Text)
                        })
                        .await
                        .unwrap();
                }
            }

            match guild.channel_id_from_name(ctx.clone(), STOCK_CHANNEL).await {
                Some(_) => {}
                None => {
                    guild
                        .create_channel(ctx.clone(), |c| {
                            c.name(STOCK_CHANNEL).kind(ChannelType::Text)
                        })
                        .await
                        .unwrap();
                }
            }
        }
    }

    async fn message(&self, ctx: Context, msg: Message) {
        let guild_id = msg.guild_id.unwrap();
        let guild = guild_id.to_guild_cached(ctx.clone()).await.unwrap();

        let shop_channel_id = guild
            .channel_id_from_name(ctx.clone(), SHOP_CHANNEL)
            .await
            .unwrap();
        let message_channel_id = msg.channel_id;

        if shop_channel_id == message_channel_id {
            if !msg.is_own(ctx.clone()).await {
                let content = msg.content.clone();
                shop_channel_id.say(ctx.clone(), content).await.unwrap();
                msg.delete(ctx).await.unwrap();
            }
        }

        match msg.clone().referenced_message {
            Some(referenced_message) => {
                println!("{:?}", referenced_message);
                self.reply(msg, *referenced_message).await
            }
            None => println!("Received message: {}", msg.content),
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("!")) // set the bot's prefix to "~"
        .group(&GENERAL_GROUP);

    // Login with a bot token from the environment
    let token = env::var("DISCORD_TOKEN").expect("Missing DISCORD_TOKEN");
    let application_id = env::var("APPLICATION_ID")
        .expect("Missing APPLICATION_ID")
        .parse::<u64>()
        .unwrap();

    let mut client = Client::builder(token)
        .application_id(application_id)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Error creating client");

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(ctx, "Pong!").await?;
    let guild_id = msg.guild_id.unwrap();
    let guild = guild_id.to_guild_cached(ctx).await.unwrap();

    for (_, guild_channel) in guild.channels {
        println!("{}", guild_channel.name);
    }
    Ok(())
}

#[command]
async fn echo(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(ctx, msg.content.clone()).await?;

    Ok(())
}

#[command]
async fn shop(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let guild = guild_id.to_guild_cached(ctx).await.unwrap();

    let channel = guild.channel_id_from_name(ctx, SHOP_CHANNEL).await;
    let content = parse_shop_message(msg.content.clone());

    match channel {
        Some(channel_id) => {
            channel_id.say(ctx, content).await.unwrap();
        }
        None => {
            let channel = guild
                .create_channel(ctx, |c| c.name(SHOP_CHANNEL).kind(ChannelType::Text))
                .await
                .unwrap();

            channel.say(ctx, content).await.unwrap();
        }
    }

    Ok(())
}

fn parse_shop_message(content: String) -> String {
    content.strip_prefix("!shop ").unwrap().to_string()
}
