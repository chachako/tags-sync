use std::{
    fmt,
    fmt::{Debug, Formatter},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{bail, Context as ResultContext, Result};
use git2::{BranchType, Diff, Repository, Signature};
use log::debug;
use octocrab::{models::repos::Tag, repos::RepoHandler, Octocrab};
use regex::Regex;
use reqwest::Url;

use crate::{
    consts::*,
    get_env,
    utils::{github_api, github_token, CommitInfo, RepoExt},
    RepoHandlerExt,
};

/// Global context of the project.
pub struct Context {
    /// Owner of the base repository.
    base_repo_owner: String,
    /// Name of the base repository.
    base_repo_name: String,

    /// Owner of the head repository.
    head_repo_owner: String,
    /// Name of the head repository.
    head_repo_name: String,

    /// Local clone path for the head repository.
    clone_path: PathBuf,

    /// Filter tags by regular expression.
    filter_tags: Regex,
    /// URL of patch file to apply to the head repository.
    patch_file_url: Url,
    /// GitHub API client.
    github_api: Octocrab,
    /// File with all tags are stored.
    new_tags_file: PathBuf,
    /// Bash commands to be executed after each new branch is pushed.
    commands_after_sync: String,
}

impl Context {
    pub fn new() -> Result<Self> {
        fn parse_repo(value: String) -> Result<(String, String)> {
            let repo = value.split('/').collect::<Vec<_>>();
            if repo.len() != 2 {
                bail!("'{}' must be in format 'owner/repo'.", value);
            }
            Ok((
                repo.first().unwrap().to_string(),
                repo.last().unwrap().to_string(),
            ))
        }

        let github_workspace = get_env!("GITHUB_WORKSPACE");
        let github_workspace_path = Path::new(&github_workspace);

        let (base_repo_owner, base_repo_name) = parse_repo(get_env!("BASE_REPO"))?;
        let (head_repo_owner, head_repo_name) = parse_repo(get_env!("HEAD_REPO"))?;

        let result = Self {
            base_repo_owner,
            head_repo_owner,
            base_repo_name,
            head_repo_name,
            github_api: github_api()?,
            filter_tags: Regex::new(&get_env!("FILTER_TAGS"))?,
            patch_file_url: Url::parse(&get_env!("PATCH_URL"))?,
            clone_path: github_workspace_path.join(&get_env!("CLONED_PATH")),
            new_tags_file: github_workspace_path.join("new_tags.txt"),
            commands_after_sync: get_env!("COMMANDS_AFTER_SYNC"),
        };

        debug!("Load configuration {:#?}", &result);

        Ok(result)
    }

    /// Returns the new tags in the base repository.
    ///
    /// ## Details
    ///
    /// The result is derived by checking whether the **head repository**
    /// contains the corresponding branch of the tag of the **base repository**.
    ///
    /// A corresponding branch name of a tag is in "sync-${tag_name}" format.
    /// For example, the corresponding branch of the "v1.0" tag is "sync-v1.0".
    pub async fn new_tags(&self) -> Result<Vec<Tag>> {
        let mut new_tags = Vec::new();
        let base_tags = self.base_repo().list_all_tags().await?;
        let head_branches = self.head_repo().list_all_branches().await?;
        // https://github.com/rust-lang/rust-clippy/issues/6909
        #[allow(clippy::needless_collect)]
        let head_branch_names = head_branches
            .into_iter()
            .map(|branch| branch.name)
            .collect::<Vec<_>>();

        // Add all filtered tags that we think are new
        for tag in base_tags {
            let branch_name = format!("{SYNC_PREFIX}{}", tag.name);
            if !head_branch_names.contains(&branch_name) && self.filter_tags.is_match(&tag.name) {
                new_tags.push(tag);
            }
        }

        // Save new tags to the file for use as a cache key in Github Action
        fs::write(
            &self.new_tags_file,
            new_tags
                .iter()
                .map(|tag| tag.name.as_str())
                .collect::<Vec<_>>()
                .join(",")
                .as_bytes(),
        )
        .context("Failed to write new tags to file")?;

        Ok(new_tags)
    }

    /// Sync [`new_tags`] from the base repository to the head repository as branches.
    pub async fn sync_tags(&self, new_tags: &[&str]) -> Result<()> {
        // Download the patch file to prepare for subsequent work
        let response = reqwest::get(self.patch_file_url.clone()).await?;
        let patch = response.bytes().await?;
        let diff = Diff::from_buffer(&patch)?;

        let cloned_repo = self.clone_repo().await?;
        // Make sure all tags are fetched from upstream
        cloned_repo.fetch_upstream_tags(new_tags)?;
        debug!(
            "Branches: {}",
            cloned_repo
                .branches(Some(BranchType::Local))?
                .flatten()
                .map(|(branch, _)| branch.name().unwrap().unwrap().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );

        // Checkout all the new tags as branches
        for tag in new_tags {
            cloned_repo.checkout_tag(tag)?;
            // Once the branch is synced, we can apply the patch
            // to complete any needed changes
            cloned_repo.apply_patch(&diff, self.commit_info()?)?;
            // Push all changes to the remote
            cloned_repo.push_head()?;
            // Run addition bash commands
            if !self.commands_after_sync.is_empty() {
                Command::new("bash")
                    .arg("-c")
                    .arg(&self.commands_after_sync)
                    .spawn()?
                    .wait()?;
            }
        }

        Ok(())
    }

    async fn clone_repo(&self) -> Result<Repository> {
        // Clone only if the cache does not exist, otherwise we just open
        let repo = if !self.clone_path.exists() {
            macro_rules! clone_url {
                ($name:ident) => {
                    paste::paste! {
                        self.[<$name _repo>]().get().await?.clone_url.context(format!(
                            "Failed to get clone URL for {} repository.",
                            stringify!($name)
                        ))?
                    }
                };
            }

            let head_url = clone_url!(head);
            let base_url = clone_url!(base);

            debug!("Git urls: head='{}', base='{}'", head_url, base_url);

            let repo = Repository::clone(head_url.as_str(), &self.clone_path)
                .context(format!("Failed to clone: '{head_url}'"))?;
            // Add upstream url to remote
            repo.remote(UPSTREAM, base_url.as_str())?;

            repo
        } else {
            Repository::open(&self.clone_path)?
        };

        // Set Github person access token for origin-remote
        repo.remote_set_url(
            ORIGIN,
            format!(
                "https://{}@github.com/{}/{}.git",
                github_token()?,
                get_env!("GITHUB_ACTOR"),
                self.head_repo_name
            )
            .as_str(),
        )?;

        debug!("Cloned repository path: {}", repo.path().display());

        Ok(repo)
    }

    fn base_repo(&self) -> RepoHandler {
        self.github_api
            .repos(self.base_repo_owner.clone(), self.base_repo_name.clone())
    }

    fn head_repo(&self) -> RepoHandler {
        self.github_api
            .repos(self.head_repo_owner.clone(), self.head_repo_name.clone())
    }

    fn commit_info(&self) -> Result<CommitInfo> {
        let author = Signature::now(&get_env!("PATCH_AUTHOR"), &get_env!("PATCH_AUTHOR_EMAIL"))?;
        let committer = Signature::now(
            &get_env!("PATCH_COMMITTER"),
            &get_env!("PATCH_COMMITTER_EMAIL"),
        )?;
        let message = get_env!("PATCH_MESSAGE");
        let message = if message.is_empty() {
            format!("Apply patch from {}", self.patch_file_url)
        } else {
            message
        };
        Ok((author, committer, message))
    }
}

impl Debug for Context {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct("Context")
            .field(
                "base_repo",
                &format!("{}/{}", self.base_repo_owner, self.base_repo_name),
            )
            .field(
                "head_repo",
                &format!("{}/{}", self.head_repo_owner, self.head_repo_name),
            )
            .field("clone_path", &self.clone_path)
            .field("filter_tags", &self.filter_tags)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use tempfile::tempdir;

    use super::*;
    use crate::test_async_fn;

    macro_rules! test_with_context {
        ($name:ident($context:ident)$block:block) => {
            test_async_fn!($name {
                let tmp_dir = tempdir()?;
                env::set_var("GITHUB_WORKSPACE", &tmp_dir.path().canonicalize()?.as_os_str());
                env::set_var("BASE_REPO", "rust-lang/rustlings");
                env::set_var("HEAD_REPO", "ZhangHanDong/rustlings");
                env::set_var("CLONED_PATH", "rustlings-head");
                env::set_var("FILTER_TAGS", ".*");
                env::set_var("PATCH_URL", "https://github.com/rust-lang/rustlings/compare/main...ZhangHanDong:rustlings:main.patch");
                env::set_var("COMMANDS_AFTER_SYNC", "");
                env::set_var("GITHUB_ACTOR", "chachako");
                let $context = Context::new()?;
                $block
            });
        };
    }

    test_with_context!(new_tags(context) {
        // We know that the head repository does not have any branch corresponding to
        // the tag of the base repository, so all the tags of the base repository are
        // new.
        assert_eq!(context.new_tags().await?, context.base_repo().list_all_tags().await?);
    });

    test_with_context!(clone_repo(context) {
        // First we clone it and make sure it succeeds
        let repo = context.clone_repo().await?;
        assert!(repo.path().exists());
        assert_eq!(
            repo.path().canonicalize()?,
            context.clone_path.join(".git").as_path().canonicalize()?
        );
        // Then we add a remote so that we can check whether it is cached
        repo.remote("cache", "https://github.com/rust-lang/rust.git")
            .context("Failed to add remote")?;
        // Finally, we call the clone again and make sure that it uses cache
        let repo = context.clone_repo().await.context("Failed to clone")?;
        assert!(repo.find_remote("cache").is_ok());
    });
}