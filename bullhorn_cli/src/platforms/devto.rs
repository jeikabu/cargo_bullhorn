#![cfg(feature = "devto")]

/// https://docs.forem.com/api
use crate::{post::Post, *};

type ArticleResponse = serde_json::Map<String, serde_json::Value>;
const URL: &str = "https://dev.to/api";

#[derive(serde::Serialize)]
struct Article {
    title: String,
    body_markdown: String,
    published: bool,
    canonical_url: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
    series: Option<String>,
    date: Option<String>,
}

#[derive(serde::Serialize)]
struct Body {
    article: Article,
}

impl From<Post> for Body {
    fn from(item: Post) -> Self {
        let published = item.front_matter.is_published();

        let tags = item.front_matter.tags.unwrap_or_default();
        let num_tags = tags.len();
        // Must limit to 4 tags otherwise devto returns 422: "Tag list exceed the maximum of 4 tags"
        const MAX_TAGS: usize = 4;
        let tags = tags.into_iter().take(MAX_TAGS).collect();
        if num_tags > MAX_TAGS {
            warn!("Limited to {} tags, reduced to: {:?}", MAX_TAGS, tags);
        }
        let article = Article {
            title: item.front_matter.title.clone(),
            body_markdown: item.body,
            published,
            canonical_url: item.front_matter.canonical_url,
            tags,
            series: item.front_matter.series,
            date: item.front_matter.date,
        };
        Body { article }
    }
}

pub struct Devto {
    settings: Settings,
    api_token: String,
    client: reqwest::Client,
}

impl Devto {
    pub fn new(api_token: String, settings: Settings) -> Self {
        info!("Cross-posting to devto");
        let client = reqwest::Client::new();
        Self {
            settings,
            api_token,
            client,
        }
    }

    fn compare(&self, article: &ArticleResponse, value: &String) -> bool {
        let resp_field = match self.settings.compare {
            Compare::CanonicalUrl => "canonical_url",
            Compare::Slug => panic!(),
        };
        match article.get(resp_field) {
            Some(serde_json::Value::String(resp_url)) => resp_url == value,
            _ => false,
        }
    }

    fn get_id(&self, article: &ArticleResponse) -> Option<String> {
        match article.get("id") {
            Some(serde_json::Value::String(id)) => Some(id.clone()),
            Some(serde_json::Value::Number(id)) => Some(id.to_string()),
            _ => None,
        }
    }

    pub async fn try_publish(&self, post: Post) {
        if let Err(err) = self.publish(post).await {
            error!("Failed: {}", err);
        }
    }

    async fn publish(&self, post: Post) -> Result<()> {
        let compare_val = match self.settings.compare {
            Compare::CanonicalUrl => &post.front_matter.canonical_url,
            Compare::Slug => panic!("Not supported"),
        };
        let existing_id = if let Some(compare_val) = compare_val {
            let me_articles = self
                .client
                .get(format!("{}/articles/me", URL))
                .auth(self)
                .send()
                .await;
            if let Ok(me_articles) = me_articles {
                let me_articles = me_articles.json::<Vec<ArticleResponse>>().await?;
                me_articles
                    .iter()
                    .find(|a| self.compare(a, compare_val))
                    .and_then(|a| self.get_id(&a))
            } else {
                None
            }
        } else {
            None
        };

        if let Some(ref existing_id) = existing_id {
            info!(
                "Matched existing article: id={} ({:?})",
                existing_id, compare_val
            );
        }

        let body: Body = post.into();
        if self.settings.dry {
            self.client
                .get(format!("{}/articles/me", URL))
                .query(&[("per_page", "1")])
                .auth(self)
                .send()
                .await?;
        } else if let Some(existing_id) = existing_id {
            let resp: Response = self
                .client
                .put(format!("{}/articles/{}", URL, existing_id))
                .auth(self)
                .json(&body)
                .send()
                .await?
                .json()
                .await?;
            debug!("{:?}", resp);
        } else {
            let resp = self
                .client
                .post(format!("{}/articles", URL))
                .auth(self)
                .json(&body)
                .send()
                .await?
                .json()
                .await?;
            debug!("{:?}", resp);
        }
        Ok(())
    }
}

impl RequestBuilderExt<Devto> for reqwest::RequestBuilder {
    fn auth(self, platform: &Devto) -> Self {
        self.header("api-key", platform.api_token.clone())
    }
}

#[derive(Debug, serde::Deserialize)]
struct Response {
    type_of: String,
    id: u32,
    title: String,
    description: String,
    slug: String,
    path: String,
    canonical_url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response() -> Result<()> {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("devto_response.json.zst");
        let file = std::fs::File::open(path)?;
        let _: Response = serde_json::from_slice(&zstd::decode_all(file)?)?;
        Ok(())
    }
}
