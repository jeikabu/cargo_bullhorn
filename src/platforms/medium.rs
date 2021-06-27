#![cfg(feature = "medium")]

use crate::{post::Post, *};

const URL: &str = "https://api.medium.com/v1";

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
enum ContentFormat {
    Html,
    Markdown,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
enum PublishStatus {
    Public,
    Draft,
    Unlisted,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Article {
    title: String,
    content_format: ContentFormat,
    content: String,
    tags: Option<Vec<String>>,
    canonical_url: Option<String>,
    publish_status: Option<PublishStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    license: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    notify_followers: Option<bool>,
}

impl From<Post> for Article {
    fn from(item: Post) -> Self {
        let publish_status = Some(if item.front_matter.is_published() {
            PublishStatus::Public
        } else {
            PublishStatus::Draft
        });
        Self {
            title: item.front_matter.title,
            content_format: ContentFormat::Markdown,
            content: item.body,
            tags: item.front_matter.tags,
            canonical_url: item.front_matter.canonical_url,
            publish_status,
            license: None,
            notify_followers: None,
        }
    }
}

pub struct Medium {
    settings: Settings,
    api_token: String,
    pub_id: Option<String>,
    client: reqwest::Client,
}

impl Medium {
    pub fn new(api_token: String, pub_id: Option<String>, settings: Settings) -> Self {
        info!("Cross-posting to medium");
        let client = reqwest::Client::new();
        Self {
            settings,
            api_token,
            pub_id,
            client,
        }
    }

    pub async fn try_publish(&self, post: Post) {
        if let Err(err) = self.publish(post).await {
            error!("Failed: {}", err);
        }
    }

    async fn publish(&self, post: Post) -> Result<()> {
        let resp = self
            .client
            .get(format!("{}/me", URL))
            .auth(self)
            .send()
            .await?;
        let user = resp.json::<UserResponse>().await?.data;
        info!(
            "Authenticated: {} ({} {})",
            user.username, user.name, user.id
        );
        self.find_existing(&post, &user).await?;
        if self.settings.dry {
            Ok(())
        } else {
            let body: Article = post.into();
            let resp = self
                .client
                .post(format!("{}/users/{}/posts", URL, user.id))
                .auth(self)
                .json(&body)
                .send()
                .await?;
            info!("{:?}", resp);
            Ok(())
        }
    }

    async fn find_existing(&self, post: &Post, user: &UserData) -> Result<()> {
        if self.settings.compare == Compare::CanonicalUrl {
            if let Some(canonical_url) = &post.front_matter.canonical_url {
                let feed = self
                    .client
                    .get(format!("https://medium.com/feed/@{}", user.username))
                    .send()
                    .await?
                    .bytes()
                    .await?;
                let channel = rss::Channel::read_from(&feed[..])?;
                for item in channel.items {
                    if let Some(link) = item.link {
                        let story = self.client.get(link.clone()).send().await?.text().await?;
                        if let Some(story_canonical_url) = parse_article_canonical(&story) {
                            debug!(
                                "Found canonical URL: href={:?} ({})",
                                story_canonical_url, link
                            );
                            if &story_canonical_url == canonical_url {
                                info!("Matched existing article: {}", canonical_url);
                                break;
                            }
                        }
                    }
                }
            } else {
                warn!("No canonical URL");
            }
        }
        Ok(())
    }
}

impl RequestBuilderExt<Medium> for reqwest::RequestBuilder {
    fn auth(self, platform: &Medium) -> Self {
        self.header("Authorization", format!("Bearer {}", platform.api_token))
    }
}

fn parse_article_canonical(text: &str) -> Option<String> {
    let mut reader = quick_xml::Reader::from_str(text);
    reader.trim_text(true);

    let mut buf = Vec::new();
    let mut is_head = false;
    loop {
        use quick_xml::events::Event;
        match reader.read_event(&mut buf) {
            Ok(Event::Start(ref e)) if e.name() == b"head" => {
                is_head = true;
                trace!("Entered head");
            }
            Ok(Event::End(ref e)) if is_head && e.name() == b"head" => {
                trace!("Exit head");
                is_head = false;
                break;
            }
            Ok(Event::Empty(ref e)) if is_head && e.name() == b"link" => {
                let has_canonical = e.attributes().any(|attr| {
                    if let Ok(attr) = attr {
                        attr.key == b"rel" && &*attr.value == b"canonical"
                    } else {
                        false
                    }
                });
                if has_canonical {
                    let href = e
                        .attributes()
                        .filter_map(|x| x.ok())
                        .find(|attr| attr.key == b"href")
                        .map(|attr| attr.unescape_and_decode_value(&reader));
                    return href.unwrap().ok();
                }
            }
            Ok(Event::Eof) => break,
            Ok(_e) => {}
            Err(e) => warn!("Error at position {}: {:?}", reader.buffer_position(), e),
        }
        buf.clear();
    }
    None
}

#[derive(serde::Deserialize)]
struct UserResponse {
    data: UserData,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserData {
    id: String,
    username: String,
    name: String,
    url: String,
    image_url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_article() -> Result<()> {
        use std::io::prelude::*;
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("medium_article.html");
        let mut buffer = String::new();
        let _size = std::fs::File::open(path)?.read_to_string(&mut buffer)?;
        let canonical_url = parse_article_canonical(&buffer).unwrap();
        assert_eq!(
            canonical_url,
            "https://rendered-obsolete.github.io/2021/05/03/dotnet_calli.html"
        );
        Ok(())
    }
}
