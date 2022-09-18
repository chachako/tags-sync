# New Tags Sync

> #### Keep your forks synced with new tags in upstream at any time.

## Introduction

Check any new tags in the upstream repos regularly through **GitHub Action** and checkout them into your fork repos.
Forget about the manual hassle now, everything is automatic!

---

### What's the use of this?

If you have experienced modification and distribution based on a project while maintaining updates from the upstream,
you should have experienced the pain of some ***git*** commands. The purpose of this action is to minimize your pain.

### How does this work?

Checkout the upstream new tags as new branches and ***apply*** the patch you provided to make any needed changes, and
then whatever comes next is up to your Github workflow!

## Usage

### Pre-requisites

- Create a workflow `.yml` file in your repositories `.github/workflows` directory.
  An [example workflow](#example-workflow) is available below. For more information, reference the GitHub Help
  Documentation
  for [Creating a workflow file](https://help.github.com/en/articles/configuring-a-workflow#creating-a-workflow-file).
- Both the upstream and fork must be in the Github network.

### Inputs

**`base-repository`**:

- **required**

> **Note**
>
> Base (*upstream*) repository name with owner. For example, `torvalds/linux`.

**`head-repository`**:

- **default**
    - [`context github.repository`](https://docs.github.com/en/actions/learn-github-actions/contexts#github-context)

> **Note**
>
> Head (*fork*) repository name with owner. For example, `Rust-for-Linux/linux`.

**`cloned-path`**:

- **default** - `head-repo`

> **Note**
>
> Relative path
>
under [$GITHUB_WORKSPACE](https://docs.github.com/en/actions/learn-github-actions/environment-variables#default-environment-variables)
> to clone the `head repository`.

**`filter-tags`**:

- **default** - `.*`

> **Note**
>
> Filter tags by regular expression. For example, the regex `^v[2-9]\..*` controls tags that only sync versions larger
> than **`v2`**.

**`apply-patch`**:

> **Note**
>
> URL of patch file to be applied after each tag is synced as a branch. Patch url can usually be obtained by comparing
> two branches or commits.
>
> The process of applying patch occurs before the `git push`, so if the value is empty, it means that all new branches
> will be pushed directly to the `head repository` after the tags are synced.
>
> For example, apply all commits that the "head-repository" master branch is ahead of the "base-repository" master
> branch
> to each new tag:
>
> <https://github.com/base-owner/base-repository/compare/master...head-owner:head-repository:master.patch>

> **Warning**
>
> The value based on the [URL Standard](https://url.spec.whatwg.org/).

**`patch-message`**:

- **default** - `Apply patch from ${apply-patch}`

> **Note**
>
> Commit message for `git commit` when applying patch.

**`patch-author`**:

- **default** - `github-actions[bot]`

> **Note**
>
> Author for `git commit` when applying patch.

**`patch-author-email`**:

- **default** - `github-actions[bot]@users.noreply.github.com`

> **Note**
>
> Author email for `git commit` when applying patch.

**`patch-committer`**:

- **default** - `github-actions[bot]`

> **Note**
>
> Committer for `git commit` when applying patch.

**`patch-committer-email`**:

- **default** - `github-actions[bot]@users.noreply.github.com`

> **Note**
>
> Committer email for `git commit` when applying patch.

**`scripts-after-sync`**:

> **Note**
> Shell scripts runs after each new branch is pushed. Multiple scripts separated by `#!`.
>
> For example, here are two different scripts:
>
> ```bash
> #!/bin/bash
> echo "Hello World"
> 
> 
> #!/bin/bash
> echo "Hello World 2"
> ```

**`github-token`**:

- **default** - `${{ github.token }}`

> **Note**
> Personal access token (PAT) used to fetch the repository. The PAT is configured
> with the local git config, which enables your scripts to run authenticated git
> commands. The post-job step removes the PAT.
>
> We recommend using a service account with the least permissions necessary.
> Also when generating a new PAT, select the least scopes necessary.
>
> [Learn more about creating and using encrypted secrets](https://help.github.com/en/actions/automating-your-workflow-with-github-actions/creating-and-using-encrypted-secrets)

### Example workflow

```yaml
name: Sync tags on schedule
on:
  schedule:
    - cron: '0 0 * * *'
jobs:
  hello_tags_sync:
    runs-on: ubuntu-latest
    steps:
      - name: Sync from upstream to fork
        uses: chachako/tags-sync@v1
        with:
          base-repository: torvalds/linux
          patch-author: chachako
          patch-author-email: 58068445+chachako@users.noreply.github.com
          commands-after-sync: |
            echo "Hello World!"
            git tag -a v1.0 -m "my version 1.0"
            git push origin v1.0

      - name: Open a pull request
        uses: examples/auto-create-pull-request@v1
```

## License

```
Copyright (c) 2022. Chachako

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

   https://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

In addition, if you fork this project, your forked code file must contain
the URL of the original project: https://github.com/chachako/tags-sync
```
