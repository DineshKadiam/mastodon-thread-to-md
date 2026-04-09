// SPDX-FileCopyrightText: 2026 blinry <mail@blinry.org>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::PathBuf;

use futures::{stream, StreamExt, TryStreamExt};
use markdown::Download;
use url::Url;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let first_toot_url = Url::parse(&std::env::args().nth(1).expect("Please provide a toot URL."))?;
    let thread = mastodon::fetch_thread(&first_toot_url).await?;
    let blogpost: markdown::Blogpost = thread.try_into()?;

    println!("{}", blogpost.markdown);

    // Write to index.md.
    std::fs::write("index.md", blogpost.markdown)?;

    // Download the files, if they don't exist yet.
    stream::iter(blogpost.downloads)
        .map(Ok) // make it a TryStream
        .try_for_each_concurrent(8, |Download { url, filename }| async move {
            if !PathBuf::from(&filename).exists() {
                println!("Downloading {filename}...");
                let bytes = reqwest::get(url).await?.bytes().await?;
                std::fs::write(&filename, bytes)?;
            }
            Ok::<_, anyhow::Error>(())
        })
        .await?;

    Ok(())
}

// Conversions into Markdown.
mod markdown {
    use crate::mastodon;
    use anyhow::Result;
    use std::fmt::Write as _;
    use std::{collections::HashMap, path::Path};
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

    pub struct Attachment {
        pub filename: String,
        pub description: Option<String>,
    }

    impl TryFrom<mastodon::Thread> for Blogpost {
        type Error = anyhow::Error;

        fn try_from(thread: mastodon::Thread) -> Result<Self> {
            let mut markdown = String::new();

            // Add the metadata we have.
            let mut metadata: HashMap<String, String> = HashMap::new();
            metadata.insert("title".to_string(), "TODO".to_string());
            metadata.insert("tags".to_string(), "TODO".to_string());
            metadata.insert(
                "published".to_string(),
                thread.published.format("%Y-%m-%d").to_string(),
            );
            metadata.insert("toot".to_string(), thread.toot_url.to_string());

            markdown += "---\n";
            for (key, value) in metadata {
                writeln!(markdown, "{key}: {value}")?;
            }
            markdown += "---\n\n";

            let mut downloads = vec![];
            for toot in &thread.toots {
                let toot: Self = toot.try_into()?;
                markdown += "\n\n---\n\n";
                markdown += &toot.markdown;
                downloads.extend(toot.downloads);
            }

            // Collapse more than one empty line.
            let re = regex::Regex::new(r"\n\n\n+").unwrap();
            markdown = re.replace_all(&markdown, "\n\n").to_string();

            // Remove empty lines at the end of the file.
            markdown = markdown.trim_end().to_string() + "\n";

            // Add reference to the source toot.
            writeln!(
                markdown,
                "\n---\n\nThis post has been adapted from [this Mastodon thread]({}).",
                thread.toot_url
            )?;

            Ok(Self {
                markdown,
                downloads,
            })
        }
    }

    impl TryFrom<&mastodon::Toot> for Blogpost {
        type Error = anyhow::Error;

        fn try_from(toot: &mastodon::Toot) -> Result<Self> {
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

            Ok(Self {
                markdown,
                downloads,
            })
        }
    }

    impl Attachment {
        pub fn to_markdown(&self) -> String {
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
}

// Fetching data from Mastodon.
mod mastodon {
    use anyhow::{Context, Result};
    use chrono::{DateTime, Utc};
    use megalodon::entities::status::Status;
    use url::Url;

    pub struct Thread {
        pub published: DateTime<Utc>,
        pub toot_url: Url,
        pub toots: Vec<Toot>,
    }

    #[derive(Clone)]
    pub struct Toot {
        pub html: String,
        pub attachments: Vec<Attachment>,
    }

    #[derive(Clone)]
    pub struct Attachment {
        pub url: Url,
        pub description: Option<String>,
    }

    type Client = Box<dyn megalodon::Megalodon + Send + Sync>;

    fn init_client(domain: &str) -> Result<Client> {
        let access_token = std::env::var("ACCESS_TOKEN").ok();
        megalodon::generator(
            megalodon::SNS::Mastodon,
            String::from(domain),
            access_token,
            None,
        )
        .context("Failed to initialize Mastodon API client")
    }

    pub async fn fetch_thread(first_toot_url: &Url) -> anyhow::Result<Thread> {
        let client = init_client(&first_toot_url.origin().ascii_serialization())?;

        let id = first_toot_url
            .path_segments()
            .unwrap()
            .next_back()
            .unwrap()
            .to_string();
        let first_toot = client.get_status(id.clone()).await?.json();

        // Collect toots of thread.
        let megalodon_toots = collect_thread(&client, first_toot.clone()).await?;
        let mut toots = vec![];

        for toot in megalodon_toots {
            let toot = (&toot).try_into()?;
            toots.push(toot);
        }

        Ok(Thread {
            published: first_toot.created_at,
            toot_url: first_toot_url.clone(),
            toots,
        })
    }

    async fn collect_thread(client: &Client, first_toot: Status) -> Result<Vec<Status>> {
        let account = first_toot.account.clone();
        let context = client
            .get_status_context(first_toot.id.clone(), None)
            .await?
            .json();
        let mut toots = vec![first_toot];
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
            toots.push(toot);
        }

        Ok(toots)
    }

    impl TryFrom<&Status> for Toot {
        type Error = anyhow::Error;

        fn try_from(status: &Status) -> Result<Self> {
            // Add attachments.
            let mut attachments = vec![];
            for attachment in &status.media_attachments {
                let attachment = Attachment {
                    url: Url::parse(&attachment.url)?,
                    description: attachment.description.clone(),
                };

                attachments.push(attachment);
            }

            Ok(Self {
                html: status.content.clone(),
                attachments,
            })
        }
    }
}
