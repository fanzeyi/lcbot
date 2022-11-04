use std::collections::HashMap;

use anyhow::anyhow;
use anyhow::Result;
use discord::{
    builders::SendMessage,
    model::{Channel, Event, Message, RoleId, ServerId, UserId},
    Discord,
};
use rand::{seq::SliceRandom, thread_rng};
use tracing::{error, info, warn};

use crate::config::Config;

mod leetcode;

pub struct DiscordBot {
    notify_role: HashMap<ServerId, RoleId>,
    discord: Discord,
    #[allow(dead_code)]
    config: Config,
}

impl DiscordBot {
    pub fn new(config: Config) -> Result<Self> {
        let discord = Discord::from_bot_token(&config.bot_token)?;

        let notify_role = Self::load_notify_role(&discord)?;
        tracing::debug!(?notify_role, "Loaded notify role");
        Ok(DiscordBot {
            notify_role,
            discord,
            config,
        })
    }

    fn load_notify_role(discord: &Discord) -> Result<HashMap<ServerId, RoleId>> {
        let mut notify_roles = HashMap::new();

        for server in discord.get_servers()? {
            for role in discord.get_roles(server.id)? {
                if role.name == "刷题提醒" {
                    notify_roles.insert(server.id, role.id);
                }
            }
        }

        Ok(notify_roles)
    }

    fn create_thread(&self, message: &Message, name: &str) -> Result<()> {
        tracing::debug!("creating thread...");
        self.discord
            .start_thread(message.channel_id, message.id, name)?;
        Ok(())
    }

    fn request_leetcode(&self, message: &Message) -> Result<()> {
        let is_scheduled =
            message.author.bot && message.author.id == UserId(self.config.carl_bot_id);
        let mut rng = thread_rng();
        let problems = leetcode::problems();
        let choice = problems
            .choose(&mut rng)
            .ok_or_else(|| anyhow!("unable to choose rng"))?;
        let description = leetcode::get_description(&choice.slug())
            .unwrap_or("Unable to load description".into());

        let server_id = is_scheduled
            .then(|| match self.discord.get_channel(message.channel_id) {
                Ok(Channel::Public(channel)) => Some(channel.server_id),
                Ok(Channel::Category(category)) => category.server_id,
                _ => None,
            })
            .flatten();
        let content = server_id
            .and_then(|id| self.notify_role.get(&id))
            .map(|id| format!("<@&{}> 该刷题了", id))
            .unwrap_or_default();

        let question = self
            .discord
            .send_message_ex(message.channel_id, move |msg| {
                msg.content(&content).embed(|embed| {
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
            })?;

        if is_scheduled {
            self.create_thread(&question, &format!("{} - 讨论区", choice.title()))?;
        }

        Ok(())
    }

    fn process_message(&mut self, message: Message) -> Result<()> {
        info!("Message {}: {}", message.author.name, message.content);

        if message.content.starts_with("!leetcode") {
            self.request_leetcode(&message)?;
            return Ok(());
        }

        Ok(())
    }

    pub fn runloop(mut self) -> () {
        let (mut connection, _) = self.discord.connect().expect("connect failed");

        fn log_message(event: Result<Event, discord::Error>) -> Result<Event, discord::Error> {
            tracing::debug!(?event, "received");
            event
        }

        loop {
            match log_message(connection.recv_event()) {
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
