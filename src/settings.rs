use clap::{Clap, crate_version};
use crate::*;

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
    /// Dry run (e.g. no REST POST/PUT, GraphQL mutation, etc.)
    #[clap(long)]
    pub dry: bool,
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
    #[clap(long, default_value = "$HOME/.rollout.yaml")]
    pub config: String,

    /// One or more markdown files to post
    #[clap()]
    pub posts: Vec<String>,
}

#[derive(Clap, Debug, Default)]
#[clap(version = crate_version!())]
//#[clap(setting = AppSettings::ColoredHelp)]
pub struct Opts {
    #[clap(long, requires = "hashnode-publication-id")]
    pub hashnode_api_token: Option<String>,
    #[clap(long, requires = "hashnode-api-token")]
    pub hashnode_publication_id: Option<String>,
    #[clap(long)]
    pub medium_api_token: Option<String>,
    #[clap(long, requires = "medium-api-token")]
    pub medium_publication_id: Option<String>,
    #[clap(long)]
    pub devto_api_token: Option<String>,
    #[clap(flatten)]
    pub settings: Settings,
}

const DEVTO_API_TOKEN: &str = "DEVTO_API_TOKEN";
const HASHNODE_API_TOKEN: &str = "HASHNODE_API_TOKEN";
const HASHNODE_PUBLICATION_ID: &str = "HASHNODE_PUBLICATION_ID";
const MEDIUM_API_TOKEN: &str = "MEDIUM_API_TOKEN";
const MEDIUM_PUBLICATION_ID: &str = "MEDIUM_PUBLICATION_ID";

pub fn process_config(opts: &mut Opts, config: &str) -> Result<()> {
    let config = serde_yaml::from_str::<std::collections::BTreeMap<String, String>>(config)?;
    // If None, set command line from values from config
    opts.devto_api_token = opts.devto_api_token.as_ref().or(config.get(DEVTO_API_TOKEN)).cloned();
    opts.hashnode_api_token = opts.hashnode_api_token.as_ref().or(config.get(HASHNODE_API_TOKEN)).cloned();
    opts.hashnode_publication_id = opts.hashnode_publication_id.as_ref().or(config.get(HASHNODE_PUBLICATION_ID)).cloned();
    opts.medium_api_token = opts.medium_api_token.as_ref().or(config.get(MEDIUM_API_TOKEN)).cloned();
    opts.medium_publication_id = opts.medium_publication_id.as_ref().or(config.get(MEDIUM_PUBLICATION_ID)).cloned();
    Ok(())
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