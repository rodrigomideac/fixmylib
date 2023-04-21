extern crate serde;
extern crate serde_json;

mod config;
mod db;
mod errors;
mod exiftool;
mod image_converter;
mod processor;
mod scanner;
mod time;
mod video_converter;

use env_logger::Env;

#[macro_use]
extern crate log;

use crate::config::Config;
use anyhow::Context;
use clap::Parser;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::task::JoinSet;

#[derive(Clone)]
pub struct AppContext {
    config: Arc<Config>,
    db: PgPool,
}

// references: https://github.com/launchbadge/realworld-axum-sqlx/tree/main
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    let config: Config = Config::parse();
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    let db = PgPoolOptions::new()
        .max_connections(50)
        .connect(&config.database_url)
        .await
        .context("could not connect to database_url")?;
    let ctx = AppContext {
        config: Arc::new(config),
        db,
    };
    sqlx::migrate!().run(&ctx.db).await?;
    let mut set = JoinSet::new();
    let scanner_ctx = ctx.clone();
    let processor_ctx = ctx.clone();
    set.spawn(async move { scanner::run(&scanner_ctx).await });
    set.spawn(async move { processor::run(&processor_ctx).await });
    while let Some(res) = set.join_next().await {
        let _idx = res.unwrap();
    }
    Ok(())
}
