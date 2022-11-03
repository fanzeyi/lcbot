use anyhow::Result;
use discord::{
    builders::SendMessage,
    model::{Event, Message},
    Discord,
};
use rand::{seq::SliceRandom, thread_rng};
use tracing::{error, info, warn};

use crate::config::Config;

mod leetcode;

pub struct DiscordBot {
    discord: Discord,
}

impl DiscordBot {
    pub fn new(config: Config) -> Result<Self> {
        Ok(DiscordBot {
            discord: Discord::from_bot_token(&config.bot_token)?,
        })
    }

    fn request_leetcode(
        &self,
        message: &Message,
    ) -> Option<Box<dyn FnOnce(SendMessage) -> SendMessage>> {
        if !message.content.starts_with("!leetcode") {
            return None;
        }

        let mut rng = thread_rng();
        let problems = leetcode::problems();
        let choice = problems.choose(&mut rng)?;
        let description = leetcode::get_description(&choice.slug())
            .unwrap_or("Unable to load description".into());

        Some(Box::new(move |msg| {
            msg.content(&choice.to_message()).embed(|embed| {
                embed
                    .title(&choice.title())
                    .url(&choice.url())
                    .author(|a| {
                        a.name("LeetCode")
                            .url("https://leetcode.com/")
                            .icon_url("https://leetcode.com/favicon-32x32.png")
                    })
                    .color(0x58b9ff)
                    .description(&description)
                    .fields(|fields| {
                        fields
                            .field("难度", choice.difficulty(), true)
                            .field("通过率", &choice.stats(), true)
                            .field(
                                "统计",
                                &format!("{} / {}", choice.stat.accept, choice.stat.submit),
                                true,
                            )
                    })
            })
        }))
    }

    fn process_message(&mut self, message: Message) -> Result<()> {
        if message.author.bot {
            return Ok(());
        }

        info!("Message {}: {}", message.author.name, message.content);

        if let Some(content) = self.request_leetcode(&message) {
            self.discord.send_message_ex(message.channel_id, content)?;
        }

        Ok(())
    }

    pub fn runloop(mut self) -> () {
        let (mut connection, _) = self.discord.connect().expect("connect failed");

        loop {
            match connection.recv_event() {
                Ok(Event::MessageCreate(message)) => {
                    if let Err(e) = self.process_message(message) {
                        warn!("Failed to process message: {:?}", e);
                    }
                }
                Ok(_) => {}
                Err(discord::Error::Closed(code, body)) => {
                    error!("Gateway closed on us with code {:?}: {}", code, body);
                    break;
                }
                Err(err) => error!("Receive error: {:?}", err),
            }
        }
    }
}
