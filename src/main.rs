extern crate core;

use std::{env, fs, str::FromStr};

use anyhow::Context as ResultContext;
use log::info;
use pretty_env_logger::init as init_logger;
use strum::EnumString;
use Stage::Detect;

use crate::{
    context::Context,
    utils::{Action, RepoHandlerExt},
    Stage::Sync,
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

    let stage = Stage::from_str(env::args().nth(1).unwrap().as_str());
    let config = Context::new().unwrap();
    let new_tags_file = config.github_workspace().join("new_tags.txt");
    let new_tags_file = new_tags_file.as_path();

    match stage {
        Ok(Detect) => {
            let new_tags = config
                .new_tags()
                .await
                .context("Failed to get new tags")
                .unwrap();

            // Save new tags to a file
            fs::write(new_tags_file, new_tags.join("\n").as_bytes())
                .context("Failed to write new tags to file")
                .unwrap();

            Action::set_output("new-tags-file", new_tags_file.to_str().unwrap());

            info!(
                "New tags found: '{}', prepare to sync...",
                new_tags.join(", ")
            );
        }
        Ok(Sync) => {
            let file_content = fs::read_to_string(new_tags_file)
                .context("Failed to read new tags from file")
                .unwrap();
            let new_tags = file_content.split('\n').collect::<Vec<_>>();

            config
                .sync_tags(&new_tags)
                .await
                .context("Failed to sync new tags")
                .unwrap();
            info!("Synced successfully.");
        }
        Err(e) => {
            panic!("Invalid stage: {}", e);
        }
    }
}
