use anyhow::{Context, Result};
use git2::{
    ApplyLocation, AutotagOption, Cred, Diff, FetchOptions, ProxyOptions, PushOptions,
    RemoteCallbacks, RemoteRedirect, Repository,
};
use log::{debug, log_enabled, Level::Debug};

use crate::{
    consts::*,
    utils::{github_token, CommitInfo},
};

pub trait RepoExt {
    fn fetch_upstream_tags(&self, tags: &[&str]) -> Result<()>;
    fn checkout_tag(&self, tag: &str) -> Result<()>;
    fn apply_patch(&self, diff: &Diff<'_>, commit_info: CommitInfo) -> Result<()>;
    fn push_head(&self) -> Result<()>;
}

impl RepoExt for Repository {
    fn fetch_upstream_tags(&self, tags: &[&str]) -> Result<()> {
        // Fetch only specified tags from upstream
        let refspecs = tags
            .iter()
            .map(|tag| format!("+refs/tags/{tag}:refs/tags/{SYNC_PREFIX}{tag}"))
            .collect::<Vec<_>>();

        debug!("Fetching refspecs: {}", refspecs.join(" "));

        Ok(self.find_remote(UPSTREAM)?.fetch(
            &refspecs,
            Some(FetchOptions::new().download_tags(AutotagOption::None)),
            None,
        )?)
    }

    fn checkout_tag(&self, tag: &str) -> Result<()> {
        let tag_commit = self
            .find_reference(&format!("refs/tags/{SYNC_PREFIX}{tag}"))?
            .peel_to_commit()?;

        debug!("Tag '{tag}' commit '{}'", tag_commit.id());

        let branch_name = format!("{SYNC_PREFIX}{tag}");
        let branch_ref = self
            .branch(&branch_name, &tag_commit, false)?
            .into_reference();
        let branch_ref_name = branch_ref
            .name()
            .context("Failed to get branch reference name")?;

        debug!("Checking out branch '{branch_name}'");

        self.set_head(branch_ref_name)?;
        self.checkout_head(None)?;

        if log_enabled!(Debug) {
            let head = self.head()?;
            debug!(
                "Current branch='{}', id='{}'",
                head.name().unwrap(),
                head.target().unwrap()
            );
        }

        Ok(())
    }

    fn apply_patch(&self, diff: &Diff<'_>, commit_info: CommitInfo) -> Result<()> {
        self.apply(diff, ApplyLocation::Both, None)?;

        let (author, committer, message) = commit_info;
        let tree_id = self.index()?.write_tree()?;
        let tree = self.find_tree(tree_id)?;
        let parent_commit = self.head()?.peel_to_commit()?;

        debug!("Parent commit: {}", parent_commit.id());

        // Commit all changes
        self.commit(Some("HEAD"), &author, &committer, &message, &tree, &[
            &parent_commit,
        ])?;

        Ok(())
    }

    fn push_head(&self) -> Result<()> {
        let mut callbacks = RemoteCallbacks::new();
        // Using github token
        callbacks.credentials(|_, _, _| {
            let github_token = github_token().context("Cannot get GITHUB_TOKEN").unwrap();
            Cred::userpass_plaintext(&github_token, "")
        });
        callbacks.push_update_reference(|reference, status| {
            debug!(
                "Pushed reference='{}', succeed='{}'",
                reference,
                status.is_none()
            );
            Ok(())
        });

        let mut options = PushOptions::new();
        options
            .packbuilder_parallelism(0)
            .proxy_options(proxy_auto())
            .follow_redirects(RemoteRedirect::All)
            .remote_callbacks(callbacks);

        // Push all changes from the current branch to the origin
        let head_ref = self.head()?;
        let head_ref_name = head_ref.name().unwrap();
        self.find_remote(ORIGIN)?
            .push(&[head_ref_name], Some(&mut options))?;

        Ok(())
    }
}

pub fn proxy_auto<'a>() -> ProxyOptions<'a> {
    let mut proxy = ProxyOptions::new();
    proxy.auto();
    proxy
}

#[cfg(test)]
mod tests {
    use std::{fs::File, io::prelude::*, time::SystemTime};

    use git2::*;
    use log::info;
    use tempfile::tempdir;

    use super::*;
    use crate::{test_fn, utils::RepoExt};

    test_fn!(checkout_upstream_tag {
        const EXPECTED_TAG: &str = "5.2.0";

        let temp_dir = tempdir()?.path().to_path_buf();
        let repo = Repository::clone("https://github.com/ZhangHanDong/rustlings.git", &temp_dir)?;

        assert!(temp_dir.exists());
        assert!(repo.path().exists());

        repo.remote(UPSTREAM, "https://github.com/rust-lang/rustlings.git")?;
        repo.fetch_upstream_tags(&[EXPECTED_TAG])?;

        // Make sure the tag have been fetched
        assert!(repo
            .find_reference(&format!("refs/tags/{SYNC_PREFIX}{EXPECTED_TAG}"))
            .is_ok());

        // Checkout the tag as a new branch
        repo.checkout_tag(EXPECTED_TAG)?;

        // Make sure the branch have been switched
        assert_eq!(
            repo.head()?.name(),
            Some(format!("refs/heads/{SYNC_PREFIX}{EXPECTED_TAG}").as_str())
        );
    });

    test_fn!(push_head {
        if option_env!("GITHUB_TEST").is_none() {
            info!("GITHUB_TEST is not set, skipping test");
            return Ok(());
        }

        let temp_dir = tempdir()?.path().to_path_buf();
        let repo = Repository::clone("https://github.com/chachako/git2-rs-test.git", &temp_dir)?;

        // Make some random changes
        let time = SystemTime::now();
        let time = time.duration_since(SystemTime::UNIX_EPOCH)?.as_secs().to_string();
        let mut file = File::options()
            .create(true)
            .write(true)
            .append(true)
            .open(repo.workdir().unwrap().join("test.txt"))?;
        writeln!(file, "{}", &time)?;

        let last_commit = repo.head()?.peel_to_commit()?;
        let author = last_commit.author();
        let author = Signature::now(author.name().unwrap(), author.email().unwrap())?;
        let message = format!("test for {}", time);

        // Commit changes
        let mut index = repo.index()?;
        index.add_all(&["*"], IndexAddOption::DEFAULT, None)?;
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        let parent_commit = repo.head()?.peel_to_commit()?;

        repo.commit(
            Some("HEAD"),
            &author,
            &author,
            &message,
            &tree,
            &[&parent_commit]
        )?;

        // Push changes
        repo.push_head()?;
    });
}
