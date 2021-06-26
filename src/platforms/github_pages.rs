#![cfg(feature = "github_pages")]

use crate::*;

#[derive(serde::Serialize)]
struct Article {
	title: String,
	published: bool,
	tags: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
struct FilenameParts {
	pub year: u32,
	pub month: u32,
	pub day: u32,
	pub name: String,
}

pub struct GithubPages {
	settings: Settings,
	repo: git::Repository,
}

impl GithubPages {
	pub fn new(post: &Post, settings: Settings) -> Result<Self> {
		debug!("git: Searching for repository: {:?}", post.path);
		let mut repo_path = post.path.parent();
		let (repo, repo_path) = loop {
			match repo_path {
				Some(path) =>
					match git::Repository::open(&path) {
						Ok(repo) => break (repo, path),
						_ => repo_path = path.parent(),
					},
				None => return Err(Error::NotFound { expected: format!("Git repository for: {:?}", &post.path) }.into()),
			};
		};
		debug!("git: Found repository: {:?}", &repo_path);
		Ok(Self {
			settings,
			repo,
		})
	}

	pub fn publish(&self, post: &mut Post) -> Result<()> {
		// TODO: move to `_posts/YYYY/`, add and git commit, git push

		let parts = GithubPages::parse_filename(&post)?;

		if post.front_matter.canonical_url.is_none() {
			let url = self.get_canonical_url(&parts)?;
			debug!("Setting canonical URL: {} ({:?})", url, post.path);
			post.front_matter.canonical_url = Some(url);
		}

		if post.front_matter.date.is_none() {
			// 2021-05-03 00:00:00 UTC
			let date = format!("{}-{:02}-{:02}", parts.year, parts.month, parts.day);
			debug!("Setting date: {} ({:?})", date, post.path);
			post.front_matter.date = Some(date);
		}
		
		Ok(())
	}

	fn get_canonical_url(&self, parts: &FilenameParts) -> Result<String> {
		// Obtain server from git remote.  E.g.
		// `origin	github:repo/repo.github.io.git` -> `repo.github.io`
		let origin = self.repo.find_remote(&self.settings.remote)?;
		let origin_url = origin.url().expect("Bad remote");
		let regex = regex::Regex::new(r".*/(?P<pages_url>.*\.github\.io)(\.git)?")?;
		let remote_error = Error::NotFound { expected: "repo/repo.github.io.git".to_owned() };
		let url = regex.captures(origin_url)
			.ok_or_else(|| remote_error.clone())?
			.name("pages_url").ok_or(remote_error)?;
		trace!("git: Remote server: {}", url.as_str());

		// Url format: `https://server/YYYY/MM/DD/name.html`
		let url = format!("https://{}/{}/{:02}/{:02}/{}.html", 
			url.as_str(), parts.year, parts.month, parts.day, parts.name);

		Ok(url)
	}

	fn parse_filename(post: &Post) -> Result<FilenameParts> {
		// Ignore extension and parse filename as `YYYY-MM-DD-name` (per https://jekyllrb.com/docs/structure/)
		let file_error = Error::BadPath { expected: "YYYY-MM-DD-name.ext".to_owned(), found: post.path.to_owned() };
		let file_stem = post.path.file_stem().ok_or_else(|| file_error.clone())?
			.to_str().ok_or_else(|| file_error.clone())?;
		let regex = regex::Regex::new(r"(\d{4})-(\d{1,2})-(\d{1,2})-(.*)")?;
		let captures = regex.captures(file_stem).ok_or_else(|| file_error.clone())?;
		Ok(FilenameParts {
			year: captures.get(1).unwrap().as_str().parse::<u32>()?,
			month: captures.get(2).unwrap().as_str().parse::<u32>()?,
			day: captures.get(3).unwrap().as_str().parse::<u32>()?,
			name: captures.get(4).unwrap().as_str().to_owned(),
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::path::PathBuf;

	fn create_post(filename: &str) -> Post {
		Post {
			path: PathBuf::from(format!("{}/{}", env!("CARGO_MANIFEST_DIR"), filename)),
			..Default::default()
		}
	}

	#[test]
	fn create() -> Result<()> {
		let post = create_post("2021-7-1-test.md");
		let settings: Settings = Default::default();
		GithubPages::new(&post, settings)?;
		Ok(())
	}

	#[test]
	fn parse_filename() -> Result<()> {
		let post = create_post("2021-7-1-test.md");
		let parts = GithubPages::parse_filename(&post)?;
		assert_eq!(parts, FilenameParts{ year: 2021, month: 7, day: 1, name: "test".to_owned() });

		let post = create_post("2021-07-01-test.md");
		let parts = GithubPages::parse_filename(&post)?;
		assert_eq!(parts, FilenameParts{ year: 2021, month: 7, day: 1, name: "test".to_owned() });

		let post = create_post("test.md");
		let _ = GithubPages::parse_filename(&post).unwrap_err();
		Ok(())
	}

	#[test]
	fn get_canonical_url() -> Result<()> {
		let post = create_post("2021-7-1-test.md");
		let settings = Settings {
			remote: "origin".to_owned(),
			..Default::default()
		};
		let github_pages = GithubPages::new(&post, settings)?;
		let parts = GithubPages::parse_filename(&post)?;
		let _ = github_pages.get_canonical_url(&parts).unwrap_err();
		//assert_eq!(canonical_url, "https://cargo_bullhorn.github.io/2021/07/01/test.html");
		Ok(())
	}
}