// SPDX-FileCopyrightText: 2026 blinry <mail@blinry.org>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Conversions into Markdown.

use crate::mastodon;
use std::fmt::Write as _;
use std::path::Path;
use url::Url;

/// Contains information on what to put in a directory to have a blog post!
#[derive(Debug)]
pub struct Blogpost {
    pub markdown: String,
    pub downloads: Vec<Download>,
}

/// Represents a file that needs to be downloaded.
#[derive(Debug)]
pub struct Download {
    pub url: String,
    pub filename: String,
}

/// Represents an attachment, used internally.
struct Attachment {
    filename: String,
    description: Option<String>,
}

impl From<mastodon::Thread> for Blogpost {
    fn from(thread: mastodon::Thread) -> Self {
        let mut markdown = String::new();

        // Add the metadata we have.
        markdown += "---\n";
        markdown += "title: TODO\n";
        markdown += "tags: TODO\n";
        markdown += format!("published: {}\n", thread.published.format("%Y-%m-%d")).as_str();
        markdown += format!("toot: {}\n", thread.toot_url).as_str();
        markdown += "---\n\n";

        let mut downloads = vec![];
        for toot in &thread.toots {
            let toot: Self = toot.into();
            markdown += &toot.markdown;
            markdown += "\n\n---\n\n";
            downloads.extend(toot.downloads);
        }

        // Collapse more than one empty line.
        let re = regex::Regex::new(r"\n\n\n+").expect("Should be able to construct newline regex");
        markdown = re.replace_all(&markdown, "\n\n").to_string();

        // Remove empty lines at the end of the file.
        markdown = markdown.trim_end().to_string() + "\n";

        // Add reference to the source toot.
        writeln!(
            markdown,
            "This post has been adapted from [this Mastodon thread]({}).",
            thread.toot_url
        )
        .expect("Appending to the markdown string should work");

        Self {
            markdown,
            downloads,
        }
    }
}

impl From<&mastodon::Toot> for Blogpost {
    fn from(toot: &mastodon::Toot) -> Self {
        let mut markdown = mdka::from_html(&toot.html);

        // Fix spaces after @ characters.
        markdown = markdown.replace("[@ ", "[@");

        // Remove links to hashtags.
        let re = regex::Regex::new(r"\[# (.*?)\]\(.*?\)")
            .expect("The link-parsing regex should compile");
        markdown = re.replace_all(&markdown, "#$1").to_string();

        // Add attachments.
        let mut downloads = vec![];
        for attachment in &toot.attachments {
            let url = attachment.url.clone();
            let filename = filename_for(&url);

            downloads.push(Download {
                url,
                filename: filename.clone(),
            });

            let attachment = Attachment {
                filename,
                description: attachment.description.clone(),
            };

            markdown += "\n\n";
            markdown += &attachment.to_markdown();
        }

        Self {
            markdown,
            downloads,
        }
    }
}

impl Attachment {
    fn to_markdown(&self) -> String {
        let description = self.description.clone().map(|d| {
            d.replace('\n', "")
                .replace('"', "&quot;") // This text will end up in contexts where it will be quoted with ".
                .replace('_', "&lowbar;") // Avoid markdown parsers picking up the _ pair in ![_](img.png "_").
        });
        let alt_text = description.clone();

        // Avoid markdown parsers picking up a formula in ![$$](img.png "$$").
        let description = description.map(|d| d.replace("$$", "\\$$"));

        if Path::new(&self.filename)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("mp4"))
        {
            let description =
                description.map_or_else(String::new, |text| format!(" aria-label=\"{text}\""));

            format!(
                "<video src=\"{0}\" controls{1}></video>",
                self.filename, description,
            )
        } else {
            let description = description.unwrap_or_default();

            let alt_text = alt_text.map_or_else(String::new, |text| format!(" \"{text}\""));

            format!("![{0}]({1}{2})", description, self.filename, alt_text,)
        }
    }
}

fn filename_for(url: &str) -> String {
    let filename = Url::parse(url)
        .expect("Url should have been valid")
        .path_segments()
        .unwrap()
        .next_back()
        .unwrap()
        .to_string();
    filename.replace(".jpeg", ".jpg")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correct_filename() {
        assert_eq!(
            filename_for("https://example.com/image.png"),
            "image.png".to_string()
        );
        assert_eq!(
            filename_for("https://example.com/image.jpeg"),
            "image.jpg".to_string()
        );
    }

    #[test]
    fn image_attachment() {
        assert_eq!(
            Attachment {
                filename: "cat.jpg".to_string(),
                description: Some("A cute cat.".to_string()),
            }
            .to_markdown(),
            r#"![A cute cat.](cat.jpg "A cute cat.")"#.to_string()
        );

        assert_eq!(
            Attachment {
                filename: "cat.jpg".to_string(),
                description: None,
            }
            .to_markdown(),
            "![](cat.jpg)".to_string()
        );
    }

    #[test]
    fn video_attachment() {
        assert_eq!(
            Attachment {
                filename: "cat.mp4".to_string(),
                description: Some("A cute cat.".to_string()),
            }
            .to_markdown(),
            r#"<video src="cat.mp4" controls aria-label="A cute cat."></video>"#.to_string()
        );

        assert_eq!(
            Attachment {
                filename: "cat.mp4".to_string(),
                description: None,
            }
            .to_markdown(),
            r#"<video src="cat.mp4" controls></video>"#.to_string()
        );
    }
}
