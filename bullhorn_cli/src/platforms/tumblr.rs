#![cfg(feature = "tumblr")]

/// https://www.tumblr.com/docs/en/api/v2
use crate::*;

const WWW: &str = "https://www.tumblr.com";
const URL: &str = "https://api.tumblr.com/v2";

pub struct Tumblr {
    settings: Settings,
    consumer_key: String,
    consumer_secret: String,
    token: String,
    token_secret: String,
    blog_id: String,
    client: reqwest::Client,
}

impl Tumblr {
    pub fn new(
        consumer_key: String,
        consumer_secret: String,
        token: String,
        token_secret: String,
        blog_id: String,
        settings: Settings,
    ) -> Self {
        info!("Cross-posting to tumblr");
        let client = reqwest::Client::new();
        Self {
            settings,
            consumer_key,
            consumer_secret,
            token,
            token_secret,
            blog_id,
            client,
        }
    }

    pub async fn try_publish(&self, post: post::Post) {
        if let Err(err) = self.publish(post).await {
            error!("Failed: {}", err);
        }
    }

    async fn publish(&self, post: post::Post) -> Result<()> {
        let posts: Posts = self
            .client
            .get(format!("{}/blog/{}/posts", URL, self.blog_id))
            // Only requires api_key authentication, get response in "Neue Post Format"
            .query(&[("api_key", &self.consumer_key), ("npf", &"true".to_owned())])
            .send()
            .await?
            .json()
            .await?;
        debug!("POSTS {:?}", posts);
        let existing = Self::find_existing(&post, &posts);
        if let Some(ref id) = existing {
            info!("Matched existing article: id={}", id);
        }
        // Only legacy API supports markdown, Neue Post Format (NPF) doesn't
        // https://github.com/tumblr/docs/blob/master/api.md#post--create-a-new-blog-post-legacy

        // Must authenticate using both client/consumer and user tokens/secrets
        let token = oauth1_request::Token::from_parts(
            &self.consumer_key,
            &self.consumer_secret,
            &self.token,
            &self.token_secret,
        );
        let tags = post.front_matter.tags.map(|tags| RequestTags { tags });
        let request = LinkRequest {
            // If we found existing article this will be Some and we'll update.  Otherwise this is None and we create.
            id: existing.clone(),
            title: Some(post.front_matter.title),
            //date: post.front_matter.date,
            url: post.front_matter.canonical_url.unwrap(),
            tags,
            ..Default::default()
        };
        
        // To create an article: POST {blog_id}/post
        // To update: POST {blog_id}/post/edit
        let uri = format!(
            "{}/blog/{}/post{}",
            URL,
            self.blog_id,
            if existing.is_some() { "/edit" } else { "" }
        );
        // Sign the request and create `Authorization` HTTP header
        let auth_header =
            oauth1_request::post(uri.clone(), &request, &token, oauth1_request::HmacSha1);
        // For POST, request body contains `application/x-www-form-urlencoded`
        let body = oauth1_request::to_form_urlencoded(&request);
        trace!("{}", auth_header);
        trace!("{}", body);
        if self.settings.dry {
        } else {
            let resp = self
                .client
                .post(uri)
                .header(reqwest::header::AUTHORIZATION, auth_header)
                .header(
                    reqwest::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .body(body)
                .send()
                .await?;
            debug!("{:?}", &resp);
            let text = resp.text().await?;
            debug!("{}", text);
        }

        Ok(())
    }

    fn find_existing(post: &post::Post, posts: &Posts) -> Option<String> {
        posts.response.posts.iter().find_map(|p| {
            p.content
                .iter()
                // Find block that is a "link" and contains canonical URL, and return its ID
                .find(|block| match block {
                    ContentBlock::Link { display_url, .. } => {
                        display_url == post.front_matter.canonical_url.as_ref().unwrap()
                    }
                    _ => false,
                })
                .map(|_| p.id_string.clone())
        })
    }
}

// HTTP request to create/update "link" type post
#[derive(oauth1_request::Request)]
struct LinkRequest {
    /// Must be `Some` when updating an existing article, `None` when creating a new one
    id: Option<String>,
    #[oauth1(rename = "type")]
    r#type: String,
    state: Option<String>,
    tags: Option<RequestTags>,
    date: Option<String>,
    format: Option<String>,

    title: Option<String>,
    url: String,
    description: Option<String>,
}

// Helper to serialize Vec<_>
struct RequestTags {
    tags: Vec<String>,
}

// Need to impl display so oauth1_request knows how to serialize a Vec<_>
impl std::fmt::Display for RequestTags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let tag_param = self.tags.join(",");
        write!(f, "{}", tag_param)
    }
}

impl Default for LinkRequest {
    fn default() -> Self {
        Self {
            id: None,
            r#type: "link".to_owned(),
            state: None,
            tags: None,
            date: None,
            format: None,

            title: None,
            url: String::new(),
            description: None,
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct Posts {
    meta: Meta,
    response: PostsResponse,
}

#[derive(Debug, serde::Deserialize)]
struct Meta {
    status: u32,
    msg: String,
}

#[derive(Debug, serde::Deserialize)]
struct PostsResponse {
    blog: Blog,
    posts: Vec<Post>,
    total_posts: u32,
}

#[derive(Debug, serde::Deserialize)]
struct Blog{}

#[derive(Debug, serde::Deserialize)]
struct Post {
    id: u64,
    id_string: String,
    slug: String,
    summary: String,
    content: Vec<ContentBlock>,
}

// Serde will serialize these from JSON like, e.g.:
// {type="link", display_url="xxx", ...}
#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ContentBlock {
    Link { display_url: String, title: String },
}

pub struct Auth {
    settings: Settings,
    consumer_key: String,
    consumer_secret: String,
    client: reqwest::Client,
}

impl Auth {
    pub fn new(consumer_key: String, consumer_secret: String, settings: Settings) -> Self {
        let client = reqwest::Client::new();
        Self {
            settings,
            consumer_key,
            consumer_secret,
            client,
        }
    }

    pub async fn try_auth(&self) {
        if let Err(err) = self.auth().await {
            error!("Failed: {}", err);
        }
    }

    async fn auth(&self) -> Result<()> {
        let uri = format!("{}/oauth/request_token", WWW);
        let client_credentials =
            oauth1_request::Credentials::new(&self.consumer_key, &self.consumer_secret);
        // Sign request using only client/consumer credentials
        let auth_header =
            oauth1_request::Builder::<_, _>::new(client_credentials, oauth1_request::HmacSha1)
                // Can optionally specify callback URL here, but Tumbler will use the application default
                //.callback("https://callback_url")
                .post(uri.clone(), &());
        trace!("Authorization: {}", auth_header);
        let resp = self
            .client
            .post(uri)
            .header(reqwest::header::AUTHORIZATION, auth_header)
            .send()
            .await?;
        debug!("{:?}", resp);
        let resp_body = resp.text().await?;
        // Parse `key0=value0&key1=value1&...` in response body for temporary credentials
        let mut resp_body_pairs = resp_body
            .split('&')
            .map(|pair| pair.split_once('='))
            .flatten();
        let temp_token = get_value(&mut resp_body_pairs, "oauth_token")?.to_owned();
        let temp_token_secret = get_value(&mut resp_body_pairs, "oauth_token_secret")?.to_owned();
        trace!("Temporary oauth_token: {}", temp_token);

        // Create temporary SQS queue `bullhorn-{temporary token}` to receive oauth_verifier from lambda
        let queue_name = format!("bullhorn-{}", temp_token);
        trace!("Creating SQS queue: {}", queue_name);
        let client = aws_sqs::Client::from_env();
        let output = client.create_queue().queue_name(queue_name).send().await?;
        let queue_url = output.queue_url.unwrap();
        trace!("Created SQS queue: {}", queue_url);

        // Show "resource owner" approval website in system default web browser
        let query = format!("{}/oauth/authorize?oauth_token={}", WWW, temp_token);
        let exit_status = open::that(query)?;
        // use anyhow::Context;
        // if exit_status.success() {
        //     Ok(())
        // } else if let Some(exit_code) = exit_status.code() {
        //     Err(Error::Failed).with_context(|| format!("Exit code: {}", exit_code))
        // } else {
        //     Err(Error::Failed).context("Unknown exit code")
        // }

        // Receive oauth_verifier from lambda via SQS
        let messages = loop {
            let output = client
                .receive_message()
                .queue_url(&queue_url)
                .send()
                .await?;
            if let Some(msgs) = output.messages {
                if !msgs.is_empty() {
                    break msgs;
                }
            }
        };
        // Delete the temporary SQS queue
        let _ = client.delete_queue().queue_url(queue_url).send().await?;
        let verifier = messages[0].body.as_ref().unwrap();
        trace!("Verifier: {}", verifier);

        // Exchange client/consumer and temporary credentials for user credentials
        let uri = format!("{}/oauth/access_token", WWW);
        let temp_credentials = oauth1_request::Credentials::new(&temp_token, &temp_token_secret);
        // Must authenticate with both client and temporary credentials
        let token = oauth1_request::Token::new(client_credentials, temp_credentials);
        let auth_header =
            oauth1_request::Builder::<_, _>::with_token(token, oauth1_request::HmacSha1)
                // Must include `oauth_verifier`
                .verifier(verifier.as_ref())
                .get(uri.clone(), &());
        trace!("Get access_token header: {}", auth_header);
        let resp = self.client
            .get(uri)
            .header(reqwest::header::AUTHORIZATION, auth_header)
            .send()
            .await?
            .text()
            .await?;
        debug!("Results: {}", resp);

        Ok(())
    }
}

fn get_value<'a>(
    iterator: &mut impl Iterator<Item = (&'a str, &'a str)>,
    find_key: &str,
) -> Result<&'a str> {
    iterator
        .find_map(|(key, value)| if key == find_key { Some(value) } else { None })
        .ok_or_else(|| {
            Error::NotFound {
                expected: find_key.to_owned(),
            }
            .into()
        })
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn oauth_signature() {
        let signature_regex = regex::Regex::new(".*=\"(?P<oauth_value>.*)\"").unwrap();

        //https://datatracker.ietf.org/doc/html/rfc5849#section-1.2
        let consumer_key = "dpf43f3p2l4k3l03";
        let consumer_secret = "kd94hf93k423kf44";
        let client = oauth1_request::Credentials::new(consumer_key, consumer_secret);
        let auth_header = oauth1_request::Builder::<_, _>::new(client, oauth1_request::HmacSha1)
            .callback("http://printer.example.com/ready")
            .nonce("wIjqoS")
            .timestamp(std::num::NonZeroU64::new(137131200))
            .post("https://photos.example.net/initiate", &());
        let oauth_signature = signature_regex
            // Last value should be `oauth_signature=...`
            .captures(auth_header.split(',').last().unwrap())
            .unwrap()
            .name("oauth_value")
            .unwrap();
        assert_eq!(oauth_signature.as_str(), "74KNZJeDHnMBp0EMJ9ZHt%2FXKycU%3D");
    }

    #[test]
    fn existing() -> Result<()> {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("tumblr_posts.json.zst");
        
        let file = std::fs::File::open(path)?;
        let bytes = zstd::decode_all(file)?;
        let posts: Posts = serde_json::from_slice(&bytes)?;
        let post = {
            let mut post: post::Post = Default::default();
            post.front_matter.canonical_url = Some("https://rendered-obsolete.github.io/2021/05/03/dotnet_calli.html".to_owned());
            post
        };
        assert_eq!(Tumblr::find_existing(&post, &posts).unwrap(), "655788057293963264");
        Ok(())
    }
}
