//! A lightweight wrapper around the [`octocrab`] crate.

use anyhow::Result;
use async_trait::async_trait;
use log::debug;
use octocrab::{
    models::repos::{Branch, Tag},
    repos::RepoHandler,
    Octocrab,
};

/// Requests all pages through a given `expression` and return their items.
///
/// We have been requesting from the first page until
/// the result does not have the next page.
macro_rules! all_page_items {
    ($expression:expr) => {{
        let mut page = 1u32;
        let mut items = Vec::new();
        loop {
            let page_items = $expression.page(page).per_page(100).send().await?;
            items.extend(page_items.items);
            if page_items.next.is_none() {
                break;
            }
            page += 1;
        }
        Ok(items)
    }};
}

#[async_trait]
pub trait RepoHandlerExt {
    /// Returns all tags in a given repository.
    async fn list_all_tags(&self) -> Result<Vec<Tag>>;

    /// Returns all branches of a given repository.
    async fn list_all_branches(&self) -> Result<Vec<Branch>>;
}

#[async_trait]
impl RepoHandlerExt for RepoHandler<'_> {
    async fn list_all_tags(&self) -> Result<Vec<Tag>> {
        all_page_items!(self.list_tags())
    }

    async fn list_all_branches(&self) -> Result<Vec<Branch>> {
        all_page_items!(self.list_branches())
    }
}

pub trait TagsExt {
    fn names(self) -> Vec<String>;
}

impl TagsExt for Vec<Tag> {
    fn names(self) -> Vec<String> {
        self.iter().map(|tag| tag.name.clone()).collect()
    }
}

pub struct Action;

impl Action {
    pub(crate) fn set_output(key: &str, value: &str) {
        println!("::set-output name={}::{}", key, value);
    }
}

pub fn github_api() -> Result<Octocrab> {
    let token = github_token()?;
    // Print the first three digits and the last three digits.
    debug!(
        "GitHub token: {}***{}",
        &token[..5],
        &token[token.len() - 5..]
    );
    Ok(Octocrab::builder()
        .personal_token(github_token()?)
        .build()?)
}

pub fn github_token() -> Result<String> {
    Ok(get_env!("GITHUB_TOKEN"))
}

#[cfg(test)]
mod tests {
    use crate::{test_async_fn, utils::RepoHandlerExt};

    macro_rules! repo {
        () => {
            super::github_api()?.repos("vuejs", "vue")
        };
    }

    test_async_fn!(list_all_tags {
        const EXPECTED_TAGS: [(&str, &str); 5] = [
            ("v2.7.10", "ee57d9fd1d51abe245c6c37e6f8f2d45977b929e"),
            ("v2.1.6", "57f425ef1d1d5ddc89e2a9d2bbe4cfd9554fddbc"),
            ("v1.0.23-csp", "e61005f44e09199bc51c3df3eac7bd7a064d1ede"),
            ("0.11.8", "6c841059d2893d383befeed0caf8090d5f0e8b88"),
            ("0.6.0", "218557cdec830a629252f4a9e2643973dc1f1d2d"),
        ];
        let tags = repo!().list_all_tags().await?;
        // Make sure the results contain all expected tags
        for (name, sha) in EXPECTED_TAGS.iter() {
            assert!(tags.iter().any(|t| t.name == *name && t.commit.sha == *sha));
        }
    });

    test_async_fn!(list_all_branches {
        const EXPECTED_BRANCHES: [(&str, &str, bool); 3] = [
            ("0.11", "d257c81a5889d45012f6df39873fba3f8697f0cc", false),
            ("main", "60d268c4261a0b9c5125f308468b31996a8145ad", false),
            ("weex", "2acc12c9edb03329c4d9cddcca26e46c672a77bc", false),
        ];
        let tags = repo!().list_all_branches().await?;
        // Make sure the results contain all expected branches
        for (name, sha, protected) in EXPECTED_BRANCHES.iter() {
            assert!(tags.iter().any(|t|
                t.name == *name && t.commit.sha == *sha && t.protected == *protected
            ));
        }
    });
}
