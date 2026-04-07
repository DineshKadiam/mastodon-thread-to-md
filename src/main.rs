// SPDX-FileCopyrightText: 2026 blinry <mail@blinry.org>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::{Context, Result};
use futures::{stream, StreamExt, TryStreamExt};
use megalodon::entities::status::Status;
use std::path::PathBuf;
use url::Url;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let start_toot = Url::parse(&std::env::args().nth(1).expect("Please provide a toot URL."))?;
    let blogpost = convert_entire_thread(&start_toot).await?;

    println!("{}", blogpost.markdown);

    // Write to index.md.
    std::fs::write("index.md", blogpost.markdown)?;

    // Download the files, if they don't exist yet.
    stream::iter(blogpost.files)
        .map(Ok) // make it a TryStream
        .try_for_each_concurrent(8, |(url, filename)| async move {
            if !filename.exists() {
                println!("Downloading {:?}...", filename);
                let bytes = reqwest::get(url.as_str()).await?.bytes().await?;
                std::fs::write(&filename, bytes)?;
            }
            Ok::<_, anyhow::Error>(())
        })
        .await?;

    Ok(())
}

#[derive(Debug)]
struct Blogpost {
    markdown: String,
    // URL and target filename.
    files: Vec<(Url, PathBuf)>,
}

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

async fn convert_entire_thread(start_toot: &Url) -> anyhow::Result<Blogpost> {
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
    let mut files = first_toot.files;

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
        files.extend(toot.files);
    }

    // Collapse more than one empty line.
    let re = regex::Regex::new(r"\n\n\n+").unwrap();
    markdown = re.replace_all(&markdown, "\n\n").to_string();

    // Remove empty lines at the end of the file.
    markdown = markdown.trim_end().to_string() + "\n";

    // Add reference to the source toot.
    markdown +=
        format!("\n---\n\nThis post has been adapted from [this Mastodon thread]({start_toot}).\n")
            .as_str();

    Ok(Blogpost { markdown, files })
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
        let filename = filename_for(&Url::parse(&attachment.url)?);

        let description = attachment.description.clone().map(|d| {
            d.replace("\n", "")
                .replace("\"", "&quot;") // This text will end up in contexts where it will be quoted with ".
                .replace("_", "&lowbar;") // Avoid markdown parsers picking up the _ pair in ![_](img.png "_").
        });
        let description = description.unwrap_or("".to_string());

        let alt_text = format!(" \"{}\"", description);

        // Avoid markdown parsers picking up a formula in ![$$](img.png "$$").
        let description = description.replace("$$", "\\$$");

        if filename.to_str().unwrap().ends_with(".mp4") {
            markdown += &format!(
                "\n\n<video src=\"{0}\" controls aria-label=\"{1}\"></video>",
                filename.to_str().unwrap(),
                description,
            );
        } else {
            markdown += &format!(
                "\n\n![{0}]({1}{2})",
                description,
                filename.to_str().unwrap(),
                alt_text,
            );
        }

        files.push((Url::parse(&attachment.url)?, filename));
    }

    Ok(Blogpost { markdown, files })
}

fn filename_for(url: &Url) -> PathBuf {
    let mut filename = PathBuf::from(
        url.path_segments()
            .unwrap()
            .next_back()
            .unwrap()
            .to_string(),
    );
    let extension = filename.extension().unwrap().to_str().unwrap();
    if extension == "jpeg" {
        filename.set_extension("jpg");
    }
    filename
}
