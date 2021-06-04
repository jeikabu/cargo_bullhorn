use crate::{*, post::Post};

type ArticleResponse = serde_json::Map<String, serde_json::Value>;

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
		let article = Article {
			title: item.front_matter.title,
			body_markdown: item.body,
			published: true,
			canonical_url: item.front_matter.canonical_url,
			tags: item.front_matter.tags.unwrap_or(vec![]),
			series: item.front_matter.series,
			date: item.front_matter.date,
		};
		Body {
			article
		}
	}
}

pub struct DevtoCrossPublish<'s> {
	settings: &'s Settings,
	api_token: String,
	client: reqwest::Client,
}

impl<'s> DevtoCrossPublish<'s> {
	pub fn new(api_token: String, settings: &'s Settings) -> Self {
		let client = reqwest::Client::new();
		Self {
			settings,
			api_token,
			client
		}
	}

	fn compare(&self, article: &ArticleResponse, value: &String) -> bool {
		let resp_field = match self.settings.compare {
			Compare::CanonicalUrl => "canonical_url",
		};
		match article.get(resp_field) {
			Some(serde_json::Value::String(resp_url)) => 
				resp_url == value,
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

	pub async fn publish(&self, post: Post) -> Result<()> {
		let compare_val = match self.settings.compare {
			Compare::CanonicalUrl => &post.front_matter.canonical_url,
		};
		let existing_id = if let Some(compare_val) = compare_val {
			let me_articles = self.client.get("https://dev.to/api/articles/me")
				.header("api-key", &self.api_token)
				.send()
				.await;
			if let Ok(me_articles) = me_articles {
				let me_articles = me_articles.json::<Vec<ArticleResponse>>().await?;
				me_articles.iter().find(|a| self.compare(a, compare_val))
					.and_then(|a| self.get_id(&a))
			} else {
				None
			}
		} else {
			None
		};

		if let Some(ref existing_id) = existing_id {
			info!("devto: Matched existing article id: {} ({:?})", existing_id, compare_val);
		}
		
		let body: Body = post.into();
		if self.settings.dry {
			self.client.get("https://dev.to/api/articles/me")
				.query(&[("per_page", "1")])
				.header("api-key", &self.api_token)
				.send()
				.await?;
		} else {
			if let Some(existing_id) = existing_id {
				self.client.put(format!("https://dev.to/api/articles/{}", existing_id))
					.header("api-key", &self.api_token)
					.json(&body)
					.send()
					.await?;
			} else {
				self.client.post("https://dev.to/api/articles")
				.header("api-key", &self.api_token)
				.json(&body)
				.send()
				.await?;
			}
		}
		Ok(())
	}
}