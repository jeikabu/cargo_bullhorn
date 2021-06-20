use crate::{*, post::Post};
use graphql_client::GraphQLQuery;

const URL: &str = "https://api.hashnode.com/";

#[derive(graphql_client::GraphQLQuery)]
#[graphql(
	schema_path = "hashnode_schema.json",
	query_path = "src/hashnode.graphql",
	response_derives = "Debug"
	)]
pub struct Tags;

#[derive(graphql_client::GraphQLQuery)]
#[graphql(
	schema_path = "hashnode_schema.json",
	query_path = "src/hashnode.graphql",
	response_derives = "Debug",
	variables_derives = "Default"
	)]
pub struct CreatePubStory;

pub struct Hashnode {
	settings: Settings,
	api_token: String,
	pub_id: String,
	client: reqwest::Client,
}

impl Hashnode {
	pub fn new(api_token: String, pub_id: String, settings: Settings) -> Self {
		info!("Cross-posting to hashnode");
		let client = reqwest::Client::new();
		Self {
			settings,
			api_token,
			pub_id,
			client
		}
	}

	async fn tags(&self, post: &Post) -> Result<Vec<String>> {
		let mut tags: Vec<String> = vec![];
		if let Some(front_matter_tags) = &post.front_matter.tags {
			// Get all hashnode tags
			let body = Tags::build_query(tags::Variables);
			let resp = self.client.post(URL)
				.json(&body)
				.send()
				.await?;
			let categories: Vec<tags::TagsTagCategories> = {
				// Response is `[Tags]`, but `[Tags!]!`
				let categories: graphql_client::Response<tags::ResponseData> = resp.json().await?;
				categories.data
					.and_then(|d| d.tag_categories)
					// Turn `Option<Vec<Option<TagsTagCategories>>>` into `Vec<TagsTagCategories>`
					.unwrap_or_default()
					.into_iter()
					// Unwrap Some and remove None
					.filter_map(|c| c)
					.collect()
			};

			for tag in front_matter_tags {
				// Find handnode tag that matches front-matter tag
				let slug = slug::slugify(&tag);
				let tag = tag.to_lowercase();
				if let Some(tag_match) = categories.iter().find(|category|
					category.slug == slug || category.name.to_lowercase() == tag
				) {
					debug!("Matched tag `{}`: {} ({})", tag, tag_match.name, tag_match.id);
					tags.push(tag_match.id.clone());
				}
			}
		}
		Ok(tags)
	}

	pub async fn try_publish(&self, post: Post) {
		if let Err(err) = self.publish(post).await {
			error!("Failed: {}", err);
		}
	}

	async fn publish(&self, post: Post) -> Result<()> {
		let is_republished = post.front_matter.canonical_url
			.as_ref()
			.and_then(|url| Some(
				create_pub_story::isRepublished{ original_article_url: url.to_owned() }
			));
		let tags = self.tags(&post).await?
			.iter().map(|id| Some(
				create_pub_story::TagsInput{ id: id.to_owned(), name: None, slug: None }
			)).collect();
		let input = create_pub_story::CreateStoryInput {
			content_markdown: post.body,
			cover_image_url: None,
			is_anonymous: None,
			is_republished,
			slug: Some(slug::slugify(&post.front_matter.title)),
			sourced_from_github: None,
			tags,
			title: post.front_matter.title,
		};
		let body = CreatePubStory::build_query(create_pub_story::Variables {
			input,
			pub_id: self.pub_id.clone(),
			..Default::default()
		});
		debug!("REQ {:?}", serde_json::to_string_pretty(&body));

		if self.settings.dry {

		} else {
			let resp = self.client.post(URL)
			.header("Authorization", &self.api_token)
			.json(&body)
			.send()
			.await?;

			let resp: graphql_client::Response<create_pub_story::ResponseData> = resp.json().await?;
			debug!("{:?}", resp.data);
		}
		
		Ok(())
	}
}