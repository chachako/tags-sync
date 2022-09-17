extern crate core;

use anyhow::Context as ResultContext;
use log::info;
use pretty_env_logger::init as init_logger;

use crate::{context::Context, utils::RepoHandlerExt};

mod consts;
mod context;
mod utils;

#[tokio::main]
async fn main() {
    init_logger();

    let config = Context::new().unwrap();

    // First, we quickly compare with Github API to ensure that there are new tags
    let new_tags = config
        .new_tags()
        .await
        .context("Failed to get new tags")
        .unwrap();
    let new_tags = new_tags
        .iter()
        .map(|tag| tag.name.as_str())
        .collect::<Vec<_>>();

    // Then we can synchronize them or exit
    if new_tags.is_empty() {
        info!("Nothing to sync.");
    } else {
        info!("New tags found: '{}', syncing...", new_tags.join(", "));
        config
            .sync_tags(&new_tags)
            .await
            .context("Failed to sync tags")
            .unwrap();
        info!("Synced successfully.");
    }
}
