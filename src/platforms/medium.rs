use crate::{*, post::Post};

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
struct MediumPost {
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

impl From<Post> for MediumPost {
	fn from(item: Post) -> Self {
		Self {
			title: item.front_matter.title,
			content_format: ContentFormat::Markdown,
			content: item.body,
			tags: item.front_matter.tags,
			canonical_url: item.front_matter.canonical_url,
			publish_status: Some(PublishStatus::Public),
			license: None,
			notify_followers: None,
		}
	}
}

pub struct Medium<'s> {
	settings: &'s Settings,
	api_token: String,
	pub_id: String,
	client: reqwest::Client,
}

impl<'s> Medium<'s> {
	pub fn new(api_token: String, pub_id: String, settings: &'s Settings) -> Self {
		let client = reqwest::Client::new();
		Self {
			settings,
			api_token,
			pub_id,
			client,
		}
	}

	pub async fn publish(&self, post: Post) -> Result<()> {
		let body: MediumPost = post.into();
		let resp = self.client.get(format!("{}/me", URL))
			.header("Authorization", format!("Bearer {}", self.api_token))
			.send()
			.await?;
		let user = resp.json::<UserResponse>().await?.data;
		info!("Authenticated: {} ({} {})", user.username, user.name, user.id);
		let feed = self.client.get(format!("https://medium.com/feed/@{}", user.username))
			.send()
			.await?
			.bytes()
			.await?;
		let channel = rss::Channel::read_from(&feed[..])?;
		for item in channel.items {
			if let Some(link) = item.link {
				let story = self.client.get(link)
					.send().await?
					.text().await?;
				if let Some(canonical_url) = get_canonical(&story) {
					info!("Matched existing article: ({})", canonical_url);
				}
			}
		}
		if self.settings.dry {
			Ok(())
		} else {
			let resp = self.client.post(format!("{}/users/{}/posts", URL, user.id))
				.header("Authorization", format!("Bearer {}", self.api_token))
				.json(&body)
				.send()
				.await?;
			info!("{:?}", resp);
			Ok(())
		}
	}
}

fn get_canonical(text: &str) -> Option<String> {
	
	//let story: Html = quick_xml::de::from_str(text).unwrap();
	let mut reader = quick_xml::Reader::from_str(text);
	reader.trim_text(true);
	
	// if let Some(Workaround::Link(link)) = story.head.links.iter()
	// 	.find(|link| match link {
	// 		Workaround::Link(link) => link.rel == "canonical",
	// 		_ => false,
	// 	}) {
	// 	info!("FOUND! {}", link.rel);
	// }
	let mut buf = Vec::new();
	let mut is_head = false;
	loop {
		use quick_xml::events::{Event, BytesEnd, BytesStart};
		match reader.read_event(&mut buf) {
			Ok(Event::Start(ref e)) if e.name() == b"head" => {
				is_head = true;
				trace!("Entered head");
			},
			Ok(Event::End(ref e)) if is_head && e.name() == b"head" => {
				trace!("Exit head");
				is_head = false;
				break;
			},
			Ok(Event::Empty(ref e)) if is_head && e.name() == b"link" => {
				//trace!("Empty link {:?}", e);
				let has_canonical = e.attributes().find(|attr|
					if let Ok(attr) = attr {
						attr.key == b"rel" && &*attr.value == b"canonical"
					} else {
						false
					}
				).is_some();
				if has_canonical {
					let href = e.attributes().filter_map(|x| x.ok())
						.find(|attr| attr.key == b"href")
						.map(|attr| attr.unescape_and_decode_value(&reader));
					trace!("{:?}", href);
					return href.unwrap().ok();
				}
			},
			Ok(Event::Eof) => break,
			Ok(e) => {}, //trace!("Event {:?}", e),
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

#[derive(Debug, serde::Deserialize, PartialEq)]
struct Html {
    head: Head,
}

#[derive(Debug, serde::Deserialize, PartialEq)]
struct Head {
    title: String,
    links: Vec<Workaround>,
}

#[derive(Debug, serde::Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum Workaround {
	Link(Link),
	Script,
}

#[derive(Debug, serde::Deserialize, PartialEq)]
struct Link {
    rel: String,
    href: String,
    sizes: Option<String>,
}
