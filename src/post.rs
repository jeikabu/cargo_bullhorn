use serde::{Deserialize, Serialize};
use anyhow::Result;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct FrontMatter {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct Post {
    pub front_matter: FrontMatter,
    pub body: String,
    pub path: std::path::PathBuf,
}

impl Post {
    pub fn new(text: &str) -> Result<Self> {
        Self::from_string(text.to_owned())
    }

    pub fn from_string(text: String) -> Result<Self> {
        // Multi-line mode, at least 2 dashes, any trailing white-space
        let re = regex::Regex::new(r"(?m)^--+\s*")?;
        // Split at dashes into: before front-matter (nothing), front-matter, and body
        let mut matches = re.splitn(&text, 3);
        matches.next(); // Skip the split before the first dashes
        let front_matter = if let Some(fm) = matches.next() {
            serde_yaml::from_str(fm)?
        } else {
            Default::default()
        };
        let body = matches.next().and_then(|s| Some(s.to_owned())).unwrap_or_default();
        Ok(Post {
            body,
            front_matter,
            path: Default::default(),
        })
    }

    pub fn open(path: std::path::PathBuf) -> Result<Self> {
        use std::io::prelude::*;

        let mut text = String::new();
        let mut file = std::fs::File::open(&path)?;
        file.read_to_string(&mut text)?;

        Ok(Self {
            path,
            ..Self::from_string(text)?
        })
    }

    pub fn to_string(&self) -> Result<String> {
        let mut str = serde_yaml::to_string(&self.front_matter)?;
        str.push_str("---\n");
        str.push_str(&self.body);
        Ok(str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_string() {
        let text = "---\ntitle: title\n---\nbody";
        let p = Post::new(text).unwrap();
        assert_eq!(p.to_string().unwrap(), text);
    }

    #[test]
    fn missing() {
        for text in &[
            // No front-matter
            "---\n \n---\nbody", "---\n\n---\nbody", "---\n---\nbody",
            // No body
            "---\ntitle: title\n---",
            // Nothing
            ""
            ] {
            let _ = Post::new(text);
        }
    }

    #[test]
    fn lf() {
        let text = "--- \n\
            title: title \n\
            --- \n\
            body";
        let post = Post::new(text).unwrap();
        assert_eq!(post.front_matter.title, "title");
        assert_eq!(post.body, "body");
    }

    #[test]
    fn crlf() {
        let text = "--- \r\n\
            title: title \r\n\
            --- \r\n\
            body";
        let post = Post::new(text).unwrap();
        println!("{}\n{} {}", text, post.front_matter.title, post.body);
        assert_eq!(post.front_matter.title, "title");
        assert_eq!(post.body, "body");
    }

    #[test]
    fn mixed_line_endings() {
        let text = "--- \n\
            title: title \r\n\
            --- \r\n\
            body";
        let post = Post::new(text).unwrap();
        assert_eq!(post.front_matter.title, "title");
        assert_eq!(post.body, "body");
    }

    #[test]
    fn all_fields() {
        let text = "---
title: title
canonical_url: https://server.io/canonical/url.html
tags: [tag0]
series: series
---
body";
        println!("TEXT:\n{}", text);
        let post = Post::new(text).unwrap();
        println!("{}\n{} {}", text, post.front_matter.title, post.body);
        assert_eq!(post.front_matter.title, "title");
        assert_eq!(post.body, "body");
    }

    #[test]
    fn tags() {
        let no_tags = "---
title: title
tags:
---
body";
        let post = Post::new(no_tags).unwrap();
        assert_eq!(post.front_matter.tags, None);

        let one_tag = "---
title: title
tags: [tag0]
---
body";
        let post = Post::new(one_tag).unwrap();
        assert_eq!(post.front_matter.tags, Some(vec!["tag0".to_owned()]));

        let multiple_tags = Some(vec!["tag0".to_owned(), "tag1".to_owned()]);

        let list = "---
title: title
tags:
- tag0
- tag1
---
body";
        let post = Post::new(list).unwrap();
        assert_eq!(post.front_matter.tags, multiple_tags);

        let array = "---
title: title
tags: [tag0, tag1]
---
body";
        let post = Post::new(array).unwrap();
        assert_eq!(post.front_matter.tags, multiple_tags);
    }

    #[test]
    fn bad_field() {
        let text = "---
title: title
bad_field: very naughty
---
body";
        let post = Post::new(text).unwrap();
        assert_eq!(post.front_matter.title, "title");
        assert_eq!(post.body, "body");
    }

    #[test]
    fn dashes_code() {
        let text = "---
title: title
---

```
---
```";
        let post = Post::new(text).unwrap();
        assert_eq!(post.front_matter.title, "title");
        assert_eq!(post.body.is_empty(), false);
    }

    #[test]
    fn dashes_table() {
        let text = "---
title: title
---

| |
---|---
| |";
        let post = Post::new(text).unwrap();
        assert_eq!(post.front_matter.title, "title");
        assert_eq!(post.body.is_empty(), false);
    }

    #[test]
    fn comment() {
        let text = "---
title: title
# Comment
---
body
";
        let post = Post::new(text).unwrap();
        assert_eq!(post.front_matter.title, "title");
        assert_eq!(post.body.is_empty(), false);
    }
}