extern crate core;

use std::{env, str::FromStr};

use anyhow::Context as ResultContext;
use log::info;
use pretty_env_logger::init as init_logger;
use strum::EnumString;

use crate::{
    context::Context,
    utils::{Action, RepoHandlerExt},
};

mod consts;
mod context;
mod utils;

/// Multiple stages represent the execution state in Github Action.
#[derive(EnumString)]
enum Stage {
    /// Detection stage to detect whether the **base repository** has new tags
    /// that can be synchronized to the **head repository**.
    ///
    /// - Corresponding method: [`Context::new_tags`]
    Detect,

    /// Synchronization stage to synchronize tags from the **base repository**
    /// to the **head repository** as branches.
    ///
    /// - Corresponding method: [`Context::sync_tags`]
    Sync,
}

#[tokio::main]
async fn main() {
    init_logger();

    let mut args = env::args();
    let stage = args.next().unwrap();
    info!("Stage: {}", stage);
    let stage = Stage::from_str(stage.as_str());
    let config = Context::new().unwrap();

    match stage {
        Ok(Stage::Detect) => {
            let tags = config
                .new_tags()
                .await
                .context("Failed to get new tags")
                .unwrap();
            Action::set_output("new-tags", &tags.join("\n"));
            info!("New tags found: '{}', prepare to sync...", tags.join(", "));
        }
        Ok(Stage::Sync) => {
            let arg = args
                .next()
                .context("Tags arg must be present in the Sync stage")
                .unwrap();
            let tags = arg.split('\n').collect::<Vec<_>>();
            config
                .sync_tags(&tags)
                .await
                .context("Failed to sync tags")
                .unwrap();
            info!("Synced successfully.");
        }
        Err(e) => {
            panic!("Invalid stage: {}", e);
        }
    }
}
