// SPDX-FileCopyrightText: 2026 blinry <mail@blinry.org>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::PathBuf;

use blogpost::Download;
use futures::{stream, StreamExt, TryStreamExt};
use url::Url;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let start_toot = Url::parse(&std::env::args().nth(1).expect("Please provide a toot URL."))?;
    let blogpost = mastodon::convert_thread(&start_toot).await?;

    println!("{}", blogpost.markdown);

    // Write to index.md.
    std::fs::write("index.md", blogpost.markdown)?;

    // Download the files, if they don't exist yet.
    stream::iter(blogpost.downloads)
        .map(Ok) // make it a TryStream
        .try_for_each_concurrent(8, |Download { url, filename }| async move {
            if !PathBuf::from(&filename).exists() {
                println!("Downloading {:?}...", filename);
                let bytes = reqwest::get(url.as_str()).await?.bytes().await?;
                std::fs::write(&filename, bytes)?;
            }
            Ok::<_, anyhow::Error>(())
        })
        .await?;

    Ok(())
}

mod blogpost {
    use url::Url;

    #[derive(Debug)]
    pub struct Blogpost {
        pub markdown: String,
        pub downloads: Vec<Download>,
    }

    // Represents a file that needs to be downloaded.
    #[derive(Debug)]
    pub struct Download {
        pub url: Url,
        pub filename: String,
    }
}

mod markdown {
    pub struct Attachment {
        pub filename: String,
        pub description: Option<String>,
    }

    impl Attachment {
        pub fn to_markdown(&self) -> String {
            let description = self.description.clone().map(|d| {
                d.replace("\n", "")
                    .replace("\"", "&quot;") // This text will end up in contexts where it will be quoted with ".
                    .replace("_", "&lowbar;") // Avoid markdown parsers picking up the _ pair in ![_](img.png "_").
            });
            let alt_text = description.clone();

            // Avoid markdown parsers picking up a formula in ![$$](img.png "$$").
            let description = description.map(|d| d.replace("$$", "\\$$"));

            if self.filename.ends_with(".mp4") {
                let description = match description {
                    None => "".to_string(),
                    Some(text) => format!(" aria-label=\"{text}\""),
                };

                format!(
                    "<video src=\"{0}\" controls{1}></video>",
                    self.filename, description,
                )
            } else {
                let description = match description {
                    None => "".to_string(),
                    Some(text) => text,
                };

                let alt_text = match alt_text {
                    None => "".to_string(),
                    Some(text) => format!(" \"{text}\""),
                };

                format!("![{0}]({1}{2})", description, self.filename, alt_text,)
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

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
                r#"![](cat.jpg)"#.to_string()
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
}

mod mastodon {
    use crate::{
        blogpost::{Blogpost, Download},
        markdown,
    };
    use anyhow::{Context, Result};
    use megalodon::entities::status::Status;
    use url::Url;

    fn init_client(domain: &str) -> Result<Box<dyn megalodon::Megalodon + Send + Sync>> {
        let access_token = std::env::var("ACCESS_TOKEN").ok();
        megalodon::generator(
            megalodon::SNS::Mastodon,
            String::from(domain),
            access_token,
            None,
        )
        .context("Failed to initialize Mastodon API client")
    }

    pub async fn convert_thread(start_toot: &Url) -> anyhow::Result<Blogpost> {
        let client = init_client(&start_toot.origin().ascii_serialization())?;

        let mut markdown = String::new();

        let id = start_toot
            .path_segments()
            .unwrap()
            .next_back()
            .unwrap()
            .to_string();
        let toot = client.get_status(id.clone()).await?.json();

        // Add the metadata we have.
        markdown += "---\n";
        markdown += "title: TODO\n";
        markdown += "tags: TODO\n";
        markdown += format!("published: {}\n", toot.created_at.format("%Y-%m-%d")).as_str();
        markdown += format!("toot: {}\n", start_toot).as_str();
        markdown += "---\n\n";

        let first_toot = convert_toot(&toot).await?;
        markdown += &first_toot.markdown;
        let mut files = first_toot.downloads;

        let account = toot.account;

        let context = client.get_status_context(id, None).await?.json();

        for toot in context.descendants {
            // Skip replies from other users.
            if toot.account != account {
                continue;
            }

            // Skip replies to other people.
            if let Some(ref reply_to_account_id) = toot.in_reply_to_account_id {
                if reply_to_account_id != &account.id {
                    continue;
                }
            }

            let toot = convert_toot(&toot).await?;
            markdown += "\n\n---\n\n";
            markdown += &toot.markdown;
            files.extend(toot.downloads);
        }

        // Collapse more than one empty line.
        let re = regex::Regex::new(r"\n\n\n+").unwrap();
        markdown = re.replace_all(&markdown, "\n\n").to_string();

        // Remove empty lines at the end of the file.
        markdown = markdown.trim_end().to_string() + "\n";

        // Add reference to the source toot.
        markdown += format!(
            "\n---\n\nThis post has been adapted from [this Mastodon thread]({start_toot}).\n"
        )
        .as_str();

        Ok(Blogpost {
            markdown,
            downloads: files,
        })
    }

    async fn convert_toot(status: &Status) -> anyhow::Result<Blogpost> {
        let mut markdown = mdka::from_html(&status.content);

        // Fix spaces after @ characters.
        markdown = markdown.replace("[@ ", "[@");

        // Remove links to hashtags.
        let re = regex::Regex::new(r"\[# (.*?)\]\(.*?\)").unwrap();
        markdown = re.replace_all(&markdown, "#$1").to_string();

        let mut files = vec![];
        for attachment in &status.media_attachments {
            let url = Url::parse(&attachment.url)?;
            let filename = filename_for(&url);

            files.push(Download {
                url: url.clone(),
                filename: filename.clone(),
            });

            let attachment = markdown::Attachment {
                filename,
                description: attachment.description.clone(),
            };

            markdown += "\n\n";
            markdown += attachment.to_markdown().as_str();
        }

        Ok(Blogpost {
            markdown,
            downloads: files,
        })
    }

    fn filename_for(url: &Url) -> String {
        let filename = url
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
                filename_for(&Url::parse("https://example.com/image.png").unwrap()),
                "image.png".to_string()
            );
            assert_eq!(
                filename_for(&Url::parse("https://example.com/image.jpeg").unwrap()),
                "image.jpg".to_string()
            );
        }
    }
}
