use anyhow::{anyhow, Result};
use clap::{AppSettings, Clap, crate_version};
use tracing::{debug, error, info, trace, warn};
use tracing_subscriber;

mod platforms;
mod post;

use post::Post;
use platforms::*;

#[derive(thiserror::Error, Clone, Debug)]
pub enum Error {
    #[error("Bad path, expected {expected}: {found}")]
    BadPath {
        expected: String,
        found: std::path::PathBuf,
    },
    #[error("Bad string, expected {expected}: {found}")]
    BadString {
        expected: String,
        found: String,
    },
    #[error("Not found: {expected}")]
    NotFound {
        expected: String,
    },
    #[error("Bad format: {thing}")]
    BadFormat {
        thing: String,
    },
}

#[derive(clap::ArgEnum, Clone, Debug)]
pub enum Operation {
    Auto,
    Put,
    Post,
}

impl Default for Operation {
    fn default() -> Self {
        Operation::Auto
    }
}

#[derive(clap::ArgEnum, Clone, Debug)]
pub enum Compare {
    CanonicalUrl,
}


impl Default for Compare {
    fn default() -> Self {
        Compare::CanonicalUrl
    }
}

#[derive(Clap, Clone, Debug, Default)]
pub struct Settings {
    /// Dry run
    #[clap(long)]
    dry: bool,
    /// Operation to perform (i.e. update, or submit new)
    #[clap(long, arg_enum, default_value = "auto")]
    operation: Operation,
    /// How articles are compared to determine if they already exist for update
    #[clap(long, arg_enum, default_value = "canonical-url")]
    compare: Compare,
    /// Git remote to use
    #[clap(long, default_value = "origin")]
    remote: String,
    /// Publish date if not today
    #[clap(long)]
    date: Option<String>,
    #[clap(long, default_value = "$HOME/.rollout.yaml")]
    config: String,

    /// One or more markdown files to post
    #[clap()]
    posts: Vec<String>,
}

#[derive(Clap, Debug, Default)]
#[clap(version = crate_version!())]
//#[clap(setting = AppSettings::ColoredHelp)]
struct Opts {
    /// Some input. Because this isn't an Option<T> it's required to be used
    #[clap(long, requires = "hashnode-publication-id")]
    hashnode_api_token: Option<String>,
    #[clap(long, requires = "hashnode-api-token")]
    hashnode_publication_id: Option<String>,
    #[clap(long, requires = "medium-publication-id")]
    medium_api_token: Option<String>,
    #[clap(long, requires = "medium-api-token")]
    medium_publication_id: Option<String>,
    #[clap(long)]
    devto_api_token: Option<String>,
    #[clap(flatten)]
    settings: Settings,
}

const DEVTO_API_TOKEN: &str = "DEVTO_API_TOKEN";
const HASHNODE_API_TOKEN: &str = "HASHNODE_API_TOKEN";
const HASHNODE_PUBLICATION_ID: &str = "HASHNODE_PUBLICATION_ID";
const MEDIUM_API_TOKEN: &str = "MEDIUM_API_TOKEN";
const MEDIUM_PUBLICATION_ID: &str = "MEDIUM_PUBLICATION_ID";

async fn start() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let mut opts = Opts::parse();

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

        // Post "original" represented by canonical URL
        let git = github_pages::GithubPagesPublish::new(&post, &opts.settings)?;
        git.publish(&mut post)?;

        let mut futures: Vec<futures::future::LocalBoxFuture<Result<()>>> = vec![];
        if let Some(token) = &opts.devto_api_token {
            trace!("Cross-posting to devto");
            let settings = opts.settings.clone();
            let post = post.clone();
            futures.push(Box::pin(async move {
                let devto = devto::DevtoCrossPublish::new(token.clone(), &settings);
                devto.publish(post).await
            }));
        }

        if let (Some(api_token), Some(pub_id)) = (&opts.hashnode_api_token, &opts.hashnode_publication_id) {
            trace!("Posting to hashnode");
            let settings = opts.settings.clone();
            let post = post.clone();
            futures.push(Box::pin(async move {
                let hashnode = hashnode::Hashnode::new(api_token.clone(), pub_id.clone(), &settings);
                hashnode.publish(post).await
            }));
        }

        if let (Some(api_token), Some(pub_id)) = (&opts.medium_api_token, &opts.medium_publication_id) {
            trace!("Posting to medium");
            let settings = opts.settings.clone();
            let post = post.clone();
            futures.push(Box::pin(async move {
                let medium = medium::Medium::new(api_token.clone(), pub_id.clone(), &settings);
                medium.publish(post).await
            }));
        }
        let results = futures::future::join_all(futures).await;
    }
    
    Ok(())
}

fn process_config(opts: &mut Opts, config: &str) -> Result<()> {
    let config = serde_yaml::from_str::<std::collections::BTreeMap<String, String>>(config)?;
    // If None, set command line from values from config
    opts.devto_api_token = opts.devto_api_token.as_ref().or(config.get(DEVTO_API_TOKEN)).cloned();
    opts.hashnode_api_token = opts.hashnode_api_token.as_ref().or(config.get(HASHNODE_API_TOKEN)).cloned();
    opts.hashnode_publication_id = opts.hashnode_publication_id.as_ref().or(config.get(HASHNODE_PUBLICATION_ID)).cloned();
    opts.medium_api_token = opts.medium_api_token.as_ref().or(config.get(MEDIUM_API_TOKEN)).cloned();
    opts.medium_publication_id = opts.medium_publication_id.as_ref().or(config.get(MEDIUM_PUBLICATION_ID)).cloned();
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
    fn apply_config() {
        let config = format!("
            {devto}: {devto}
            {hashnode_token}: {hashnode_token}
            {hashnode_pub_id}: {hashnode_pub_id}
            ",
            devto = DEVTO_API_TOKEN,
            hashnode_token = HASHNODE_API_TOKEN,
            hashnode_pub_id = HASHNODE_PUBLICATION_ID,
        );
        let mut opts: Opts = Default::default();
        process_config(&mut opts, &config).unwrap();
        assert_eq!(opts.devto_api_token, Some(DEVTO_API_TOKEN.to_owned()));
        assert_eq!(opts.hashnode_api_token, Some(HASHNODE_API_TOKEN.to_owned()));
        assert_eq!(opts.hashnode_publication_id, Some(HASHNODE_PUBLICATION_ID.to_owned()));
    }
}