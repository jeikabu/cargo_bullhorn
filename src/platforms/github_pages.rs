use crate::*;

#[derive(serde::Serialize)]
struct Article {
	title: String,
	published: bool,
	tags: Vec<String>,
}

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
				None => return Err(anyhow!("Can't find git repository for: {:?}", &post.path)),
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
			let url = self.get_canonical_url(&post, &parts)?;
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

	fn get_canonical_url(&self, _post: &Post, parts: &FilenameParts) -> Result<String> {
		// Obtain server from git remote.  E.g.
		// `origin	github:repo/repo.github.io.git` -> `repo.github.io`
		let origin = self.repo.find_remote(&self.settings.remote)?;
		let origin_url = origin.url().expect("Bad remote");
		let regex = regex::Regex::new(r".*/(?P<pages_url>.*)\.git")?;
		let remote_error = Error::NotFound { expected: "repo/repo.github.io.git".to_owned() };
		let url = regex.captures(origin_url)
			.ok_or(remote_error.clone())?
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
		let file_stem = post.path.file_stem().ok_or(file_error.clone())?
			.to_str().ok_or(file_error.clone())?;
		let regex = regex::Regex::new(r"(\d{4})-(\d{1,2})-(\d{1,2})-(.*)")?;
		let captures = regex.captures(file_stem).expect("Bad filename");
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

	#[test]
	fn x() {

	}
}