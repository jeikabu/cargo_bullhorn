use anyhow::{anyhow, Result};
use clap::Clap;
use tracing::{debug, error, info, trace, warn};

mod platforms;
mod post;
mod settings;

use platforms::*;
use post::Post;
use settings::*;

trait RequestBuilderExt<T> {
    fn auth(self, platform: &T) -> Self;
}

#[derive(thiserror::Error, Clone, Debug)]
pub enum Error {
    #[error("Bad path, expected {expected}: {found}")]
    BadPath {
        expected: String,
        found: std::path::PathBuf,
    },
    #[error("Bad string, expected {expected}: {found}")]
    BadString { expected: String, found: String },
    #[error("Not found: {expected}")]
    NotFound { expected: String },
    #[error("Bad format: {thing}")]
    BadFormat { thing: String },
}

async fn start() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let mut opts = Opts::parse();
    if opts.platforms.is_empty() || opts.platforms.iter().any(|i| *i == Platforms::All) {
        opts.platforms.clear();
        opts.platforms.push(Platforms::Devto);
        opts.platforms.push(Platforms::Hashnode);
        opts.platforms.push(Platforms::Medium);
    }

    if let Ok(config) = shellexpand::env(&opts.settings.config) {
        use std::io::prelude::*;

        let mut buffer = String::new();
        let _size = std::fs::File::open(std::path::PathBuf::from(config.to_string()))?
            .read_to_string(&mut buffer)?;
        process_config(&mut opts, &buffer)?;
    } else {
        error!("Unable to expand config: {}", opts.settings.config);
    }

    for file in &opts.settings.posts {
        let path = std::path::PathBuf::from(file);
        if !path.is_file() {
            warn!("File not found, skipping: {}", file);
            continue;
        }
        let mut post = Post::open(path)?;
        post.apply(&opts.settings);

        // Post "original" represented by canonical URL
        #[cfg(feature = "github_pages")]
        {
            let git = github_pages::GithubPages::new(&post, opts.settings.clone())?;
            git.publish(&mut post)?;
        }

        let mut futures: Vec<futures::future::LocalBoxFuture<()>> = vec![];

        #[cfg(feature = "devto")]
        if let (Some(_), Some(api_token)) = (
            opts.platforms.iter().find(|p| **p == Platforms::Devto),
            &opts.devto_api_token,
        ) {
            let settings = opts.settings.clone();
            let post = post.clone();
            futures.push(Box::pin(async move {
                let devto = devto::Devto::new(api_token.clone(), settings);
                devto.try_publish(post).await
            }));
        }

        #[cfg(feature = "hashnode")]
        if let (Some(_), Some(api_token), Some(username)) = (
            opts.platforms.iter().find(|p| **p == Platforms::Hashnode),
            &opts.hashnode_api_token,
            &opts.hashnode_username,
        ) {
            let settings = opts.settings.clone();
            let post = post.clone();
            futures.push(Box::pin(async move {
                let hashnode =
                    hashnode::Hashnode::new(api_token.clone(), username.clone(), settings);
                hashnode.try_publish(post).await
            }));
        }

        #[cfg(feature = "medium")]
        if let (Some(_), Some(api_token)) = (
            opts.platforms.iter().find(|p| **p == Platforms::Medium),
            &opts.medium_api_token,
        ) {
            let settings = opts.settings.clone();
            let pub_id = opts.medium_publication_id.clone();
            let post = post.clone();
            futures.push(Box::pin(async move {
                let medium = medium::Medium::new(api_token.clone(), pub_id, settings);
                medium.try_publish(post).await
            }));
        }
        futures::future::join_all(futures).await;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    start().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_config() {}
}
