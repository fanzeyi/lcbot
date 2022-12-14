use config::Config;
use dotenv::dotenv;

use crate::bot::DiscordBot;

mod bot;
mod config;

fn main() {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let config = envy::from_env::<Config>().expect("Failed to load config");

    let bot = DiscordBot::new(config).unwrap();

    bot.runloop();
}
