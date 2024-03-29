use crate::*;
use clap::{crate_version, Clap};

const DEVTO_API_TOKEN: &str = "DEVTO_API_TOKEN";
const HASHNODE_API_TOKEN: &str = "HASHNODE_API_TOKEN";
const HASHNODE_USERNAME: &str = "HASHNODE_USERNAME";
const MEDIUM_API_TOKEN: &str = "MEDIUM_API_TOKEN";
const MEDIUM_PUBLICATION_ID: &str = "MEDIUM_PUBLICATION_ID";
const TUMBLR_CONSUMER_KEY: &str = "TUMBLR_CONSUMER_KEY";
const TUMBLR_CONSUMER_SECRET: &str = "TUMBLR_CONSUMER_SECRET";
const TUMBLR_OAUTH_TOKEN: &str = "TUMBLR_OAUTH_TOKEN";
const TUMBLR_OAUTH_TOKEN_SECRET: &str = "TUMBLR_OAUTH_TOKEN_SECRET";
const TUMBLR_BLOG_ID: &str = "TUMBLR_BLOG_ID";

#[derive(clap::ArgEnum, Clone, Debug, PartialEq)]
pub enum Operation {
    Auto,
    Create,
    Update,
}

impl Default for Operation {
    fn default() -> Self {
        Operation::Auto
    }
}

#[derive(clap::ArgEnum, Clone, Debug, PartialEq)]
pub enum Compare {
    CanonicalUrl,
    Slug,
}

impl Default for Compare {
    fn default() -> Self {
        Compare::CanonicalUrl
    }
}

#[derive(clap::ArgEnum, Clone, Debug, PartialEq)]
pub enum Platforms {
    Medium,
    Devto,
    Hashnode,
    Tumblr,

    All,
}

#[derive(clap::ArgEnum, Clone, Debug, PartialEq)]
pub enum UpdateField {
    Body,
    Slug,
    Tags,
}

#[derive(Clap, Clone, Debug, Default)]
pub struct Settings {
    /// Dry run (e.g. no REST POST/PUT, GraphQL mutation, etc.)
    #[clap(long)]
    pub dry: bool,
    /// Posts created as drafts, if possible
    #[clap(long)]
    pub draft: bool,
    /// Operation to perform (i.e. update, or submit new)
    #[clap(long, arg_enum, default_value = "auto")]
    pub operation: Operation,
    /// How articles are compared to determine if they already exist for update
    #[clap(long, arg_enum, default_value = "canonical-url")]
    pub compare: Compare,
    /// Git remote to use
    #[clap(long, default_value = "origin")]
    pub remote: String,
    /// Publish date if not today
    #[clap(long)]
    pub date: Option<String>,
    /// YAML file containing configuration
    #[clap(long, default_value = "$HOME/.bullhorn.yaml")]
    pub config: String,
    /// Override front-matter `slug` value
    #[clap(long)]
    pub slug: Option<String>,

    /// Article fields to write when updating an article
    #[clap(long, arg_enum, multiple = true)]
    pub update_fields: Vec<UpdateField>,

    /// One or more markdown files
    #[clap()]
    pub posts: Vec<String>,
}

#[derive(Clap, Debug, Default)]
#[clap(version = crate_version!())]
pub struct Opts {
    #[clap(long, requires = "hashnode-username", env = HASHNODE_API_TOKEN)]
    pub hashnode_api_token: Option<String>,
    #[clap(long, requires = "hashnode-api-token", env = HASHNODE_USERNAME)]
    pub hashnode_username: Option<String>,
    #[clap(long, env = MEDIUM_API_TOKEN)]
    pub medium_api_token: Option<String>,
    #[clap(long, requires = "medium-api-token", env = MEDIUM_PUBLICATION_ID)]
    pub medium_publication_id: Option<String>,
    #[clap(long, env = DEVTO_API_TOKEN)]
    pub devto_api_token: Option<String>,

    /// Tumblr consumer key (OAuth client key)
    #[clap(long, env = TUMBLR_CONSUMER_KEY)]
    pub tumblr_consumer_key: Option<String>,
    /// Tumblr consumer secret (OAuth client secret)
    #[clap(long, env = TUMBLR_CONSUMER_SECRET)]
    pub tumblr_consumer_secret: Option<String>,
    /// Tumblr user OAuth token
    #[clap(long, env = TUMBLR_OAUTH_TOKEN)]
    pub tumblr_token: Option<String>,
    /// Tumblr user OAuth token secret
    #[clap(long, env = TUMBLR_OAUTH_TOKEN_SECRET)]
    pub tumblr_token_secret: Option<String>,
    /// Tumblr blog ID (e.g. `https://www.tumblr.com/blog/{blog_id}`)
    #[clap(long, env = TUMBLR_BLOG_ID)]
    pub tumblr_blog_id: Option<String>,

    /// Platform(s) to enable.
    #[clap(long, arg_enum, multiple = true, default_value = "all")]
    pub platforms: Vec<Platforms>,

    #[clap(flatten)]
    pub settings: Settings,
}

pub fn process_config(opts: &mut Opts, config: &str) -> Result<()> {
    let config = serde_yaml::from_str::<std::collections::BTreeMap<String, String>>(config)?;
    // If None, set command line from values from config
    opts.devto_api_token = opts
        .devto_api_token
        .as_ref()
        .or_else(|| config.get(DEVTO_API_TOKEN))
        .cloned();
    opts.hashnode_api_token = opts
        .hashnode_api_token
        .as_ref()
        .or_else(|| config.get(HASHNODE_API_TOKEN))
        .cloned();
    opts.hashnode_username = opts
        .hashnode_username
        .as_ref()
        .or_else(|| config.get(HASHNODE_USERNAME))
        .cloned();
    opts.medium_api_token = opts
        .medium_api_token
        .as_ref()
        .or_else(|| config.get(MEDIUM_API_TOKEN))
        .cloned();
    opts.medium_publication_id = opts
        .medium_publication_id
        .as_ref()
        .or_else(|| config.get(MEDIUM_PUBLICATION_ID))
        .cloned();
    opts.tumblr_consumer_key = opts
        .tumblr_consumer_key
        .as_ref()
        .or_else(|| config.get(TUMBLR_CONSUMER_KEY))
        .cloned();
    opts.tumblr_consumer_secret = opts
        .tumblr_consumer_secret
        .as_ref()
        .or_else(|| config.get(TUMBLR_CONSUMER_SECRET))
        .cloned();
    opts.tumblr_token = opts
        .tumblr_token
        .as_ref()
        .or_else(|| config.get(TUMBLR_OAUTH_TOKEN))
        .cloned();
    opts.tumblr_token_secret = opts
        .tumblr_token_secret
        .as_ref()
        .or_else(|| config.get(TUMBLR_OAUTH_TOKEN_SECRET))
        .cloned();
    opts.tumblr_blog_id = opts
        .tumblr_blog_id
        .as_ref()
        .or_else(|| config.get(TUMBLR_BLOG_ID))
        .cloned();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_config() {
        let config = format!(
            "
            {devto}: {devto}
            {hashnode_token}: {hashnode_token}
            {hashnode_username}: {hashnode_username}
            ",
            devto = DEVTO_API_TOKEN,
            hashnode_token = HASHNODE_API_TOKEN,
            hashnode_username = HASHNODE_USERNAME,
        );
        let mut opts: Opts = Default::default();
        process_config(&mut opts, &config).unwrap();
        assert_eq!(opts.devto_api_token, Some(DEVTO_API_TOKEN.to_owned()));
        assert_eq!(opts.hashnode_api_token, Some(HASHNODE_API_TOKEN.to_owned()));
        assert_eq!(opts.hashnode_username, Some(HASHNODE_USERNAME.to_owned()));
    }
}
