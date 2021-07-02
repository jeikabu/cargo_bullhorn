#![cfg(feature = "hashnode")]

use crate::{post::Post, *};
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
            let resp = self.client.post(URL).json(&body).send().await?;
            let categories: Vec<tags::TagsTagCategories> = {
                // Response is GraphQL type `[Tags]` (each item and array itself can be null).
                // But `[Tags!]!` (nothing is null) is simpler in Rust
                let categories: graphql_client::Response<tags::ResponseData> = resp.json().await?;
                categories
                    .data
                    .and_then(|d| d.tag_categories)
                    // Turn `Option<Vec<Option<TagsTagCategories>>>` into `Vec<TagsTagCategories>`
                    .unwrap_or_default()
                    .into_iter()
                    .flatten()
                    .collect()
            };

            for tag in front_matter_tags {
                // Find hashnode tag that matches front-matter tag
                let slug = slug::slugify(&tag);
                if let Some(tag_match) = categories.iter().find(|category| {
                    category.slug == slug || category.name.to_lowercase() == tag.to_lowercase()
                }) {
                    debug!(
                        "Matched tag `{}`: {} ({})",
                        tag, tag_match.name, tag_match.id
                    );
                    tags.push(tag_match.id.clone());
                } else {
                    // Not returned from tag query, try GETing the tag-specific page
                    // and extracting the ID from that.;
                    let resp = self
                        .client
                        .get(format!("https://hashnode.com/n/{}", slug))
                        .send()
                        .await;
                    if let Ok(resp) = resp {
                        if let Ok(text) = resp.text().await {
                            match parse_tag_html(&text) {
                                Ok(meta) => {
                                    debug!("Matched tag `{}`: {} ({})", tag, meta.name, meta.id);
                                    tags.push(meta.id);
                                    continue;
                                }
                                Err(e) => warn!("Failed to parse tag ({}): {}", slug, e),
                            }
                        }
                    }
                    trace!("Unable to match tag: {}", tag);
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
            info!(
                "Matched existing article: id={} ({:?})",
                post_id, post.front_matter.slug
            );
            let is_republished =
                post.front_matter
                    .canonical_url
                    .as_ref()
                    .map(|url| update_story::isRepublished {
                        original_article_url: url.to_owned(),
                    });
            let is_part_of_publication = update_story::PublicationDetails { publication_id };
            let tags: Vec<Option<update_story::TagsInput>> = self
                .get_tag_ids(&post)
                .await?
                .iter()
                .map(|id| {
                    Some(update_story::TagsInput {
                        id: id.to_owned(),
                        name: None,
                        slug: None,
                    })
                })
                .collect();
            let input = update_story::UpdateStoryInput {
                title: post.front_matter.title,
                slug: post.front_matter.slug,
                content_markdown: post.body,
                cover_image_url: None,
                is_republished,
                is_part_of_publication,
                tags,
                sourced_from_github: None,
            };
            let body = UpdateStory::build_query(update_story::Variables { post_id, input });
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
            let is_republished = post.front_matter.canonical_url.as_ref().map(|url| {
                create_pub_story::isRepublished {
                    original_article_url: url.to_owned(),
                }
            });
            let tags = self
                .get_tag_ids(&post)
                .await?
                .iter()
                .map(|id| {
                    Some(create_pub_story::TagsInput {
                        id: id.to_owned(),
                        name: None,
                        slug: None,
                    })
                })
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

    async fn get_pub_and_article_id(&self, post: &Post) -> Result<(String, Option<String>)> {
        let body = PubPosts::build_query(pub_posts::Variables {
            username: self.username.clone(),
            page: 0,
        });
        let resp = self.client.post(URL).json(&body).send().await?;
        let resp: graphql_client::Response<pub_posts::ResponseData> = resp.json().await?;
        let publication = resp
            .data
            .and_then(|data| data.user)
            .and_then(|user| user.publication)
            .unwrap();
        let pub_id = publication.id;
        let existing_id = publication.posts.unwrap_or_default().iter().find_map(|p| {
            p.as_ref().and_then(|p| {
                trace!("Article: {:?}", p.slug);
                if p.slug == post.front_matter.slug {
                    Some(p.id.clone())
                } else {
                    None
                }
            })
        });
        Ok((pub_id, existing_id))
    }
}

fn parse_tag_html(text: &str) -> Result<ExtraData> {
    let mut reader = quick_xml::Reader::from_str(&text);
    reader.check_end_names(false).trim_text(true);
    let mut buf = Vec::new();
    let mut in_script = false;
    loop {
        use quick_xml::events::Event;
        match reader.read_event(&mut buf) {
            Ok(Event::Start(ref e)) if e.name() == b"script" => {
                trace!("Start {:?}", e);
                let script = e.attributes().filter_map(|attr| attr.ok()).find(|attr| {
                    attr.key == b"id"
                        && attr
                            .unescape_and_decode_value(&reader)
                            .map_or(false, |val| val == "__NEXT_DATA__")
                });
                if let Some(script) = script {
                    trace!("Script: {:?}", script);
                    in_script = true;
                }
            }
            Ok(Event::Text(ref e)) if in_script => {
                if let Ok(text) = e.unescape_and_decode(&reader) {
                    let script: Script =
                        serde_json::from_str(&text).map_err(|e| Error::BadString {
                            expected: e.to_string(),
                            found: text,
                        })?;
                    let tag_meta = script.props.page_props.extra_data;
                    trace!("Found tag {}: id={}", tag_meta.name, tag_meta.id);
                    return Ok(tag_meta);
                }
            }
            Ok(Event::End(ref e)) if in_script && e.name() == b"script" => {
                in_script = false;
            }
            Ok(Event::Eof) => break,
            Err(e) => warn!("Error at position {}: {:?}", reader.buffer_position(), e),
            _ => {}
        }
        buf.clear();
    }
    Err(Error::NotFound {
        expected: "".to_owned(),
    }
    .into())
}

#[derive(serde::Deserialize)]
struct Script {
    props: Props,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Props {
    page_props: PageProps,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageProps {
    status_code: i32,
    extra_data: ExtraData,
}

#[derive(serde::Deserialize)]
struct ExtraData {
    #[serde(rename = "_id")]
    id: String,
    name: String,
    slug: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tag() -> Result<()> {
        use std::io::prelude::*;
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .with_test_writer()
            .init();

        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("hashnode_tag.html");
        let mut buffer = String::new();
        let _size = std::fs::File::open(path)?.read_to_string(&mut buffer)?;
        let info = parse_tag_html(&buffer)?;
        assert_eq!(info.name, "dotnet");
        Ok(())
    }
}
