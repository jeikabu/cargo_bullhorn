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
    response_derives = "Debug"
)]
pub struct PubPosts;

#[derive(graphql_client::GraphQLQuery)]
#[graphql(
    schema_path = "hashnode_schema.json",
    query_path = "src/hashnode.graphql",
    response_derives = "Debug",
    variables_derives = "Default"
)]
pub struct CreatePubStory;

#[derive(graphql_client::GraphQLQuery)]
#[graphql(
    schema_path = "hashnode_schema.json",
    query_path = "src/hashnode.graphql",
    response_derives = "Debug",
    variables_derives = "Default"
)]
pub struct UpdateStory;

pub struct Hashnode {
    settings: Settings,
    api_token: String,
    username: String,
    client: reqwest::Client,
}

impl Hashnode {
    pub fn new(api_token: String, username: String, settings: Settings) -> Self {
        info!("Cross-posting to hashnode");
        let client = reqwest::Client::new();
        Self {
            settings,
            api_token,
            username,
            client,
        }
    }

	async fn get_tag_ids(&self, post: &Post) -> Result<Vec<String>> {
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
        if self.settings.compare != Compare::Slug {
            warn!(
                "Hashnode only supports comparing via Slug ({:?}), ignoring: {:?}",
                post.front_matter.slug, self.settings.compare
            );
        }
        let (publication_id, post_id) = self.get_pub_and_article_id(&post).await?;
		info!("Publication ID: {}", publication_id);
		if let Some(post_id) = post_id {
			info!("Matched existing article: id={} ({:?})", post_id, post.front_matter.slug);
            let is_republished = post.front_matter.canonical_url.as_ref().map(|url|
                update_story::isRepublished {
                    original_article_url: url.to_owned(),
                });
            let is_part_of_publication = update_story::PublicationDetails {
                publication_id,
            };
            let tags: Vec<Option<update_story::TagsInput>> = self.get_tag_ids(&post)
                .await?
                .iter()
                .map(|id|
                    Some(update_story::TagsInput {
                        id: id.to_owned(),
                        name: None,
                        slug: None,
                    })
                )
                .collect();
            let input = update_story::UpdateStoryInput {
                title: post.front_matter.title,
                slug: post.front_matter.slug,
                content_markdown: post.body,
                cover_image_url: None,
                is_republished,
                is_part_of_publication,
                tags: vec![],
                sourced_from_github: None,
            };
            let body = UpdateStory::build_query(update_story::Variables {
                input,
                post_id,
            });
            if self.settings.dry {
            } else {
                let resp = self
                    .client
                    .post(URL)
                    .header("Authorization", &self.api_token)
                    .json(&body)
                    .send()
                    .await?;
    
                let resp: graphql_client::Response<update_story::ResponseData> =
                    resp.json().await?;
                debug!("{:?}", resp.data);
            }
		} else {
            let is_republished = post.front_matter.canonical_url.as_ref().map(|url|
                create_pub_story::isRepublished {
                    original_article_url: url.to_owned(),
                });
            let tags = self.get_tag_ids(&post)
                .await?
                .iter()
                .map(|id|
                    Some(create_pub_story::TagsInput {
                        id: id.to_owned(),
                        name: None,
                        slug: None,
                    })
                )
                .collect();
            let input = create_pub_story::CreateStoryInput {
                content_markdown: post.body,
                cover_image_url: None,
                is_anonymous: None,
                is_republished,
                slug: post.front_matter.slug,
                sourced_from_github: None,
                tags,
                title: post.front_matter.title,
            };
            let body = CreatePubStory::build_query(create_pub_story::Variables {
                input,
                publication_id,
                ..Default::default()
            });
    
            if self.settings.dry {
            } else {
                let resp = self
                    .client
                    .post(URL)
                    .header("Authorization", &self.api_token)
                    .json(&body)
                    .send()
                    .await?;
    
                let resp: graphql_client::Response<create_pub_story::ResponseData> =
                    resp.json().await?;
                debug!("{:?}", resp.data);
            }
        }
        

        Ok(())
    }

    async fn get_pub_and_article_id(&self, post: &Post) -> Result<(String,Option<String>)> {
        let body = PubPosts::build_query(pub_posts::Variables { username: self.username.clone(), page: 0 });
        let resp = self.client.post(URL).json(&body).send().await?;
        let resp: graphql_client::Response<pub_posts::ResponseData> = resp.json().await?;
		let publication = resp.data
			.and_then(|data| data.user)
			.and_then(|user| user.publication)
			.unwrap();
		let pub_id = publication.id;
		let existing_id = publication.posts
			.unwrap_or_default()
			.iter()
			.find_map(|p| p.as_ref().and_then(|p| {
				trace!("Article: {:?}", p.slug);
				if p.slug == post.front_matter.slug { Some(p.id.clone()) } else { None }
			}));
		Ok((pub_id, existing_id))
    }
}
