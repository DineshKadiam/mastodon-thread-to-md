// SPDX-FileCopyrightText: 2026 blinry <mail@blinry.org>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use futures::{stream, StreamExt, TryStreamExt};
use markdown::Download;
use std::path::PathBuf;

mod markdown;
mod mastodon;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let first_toot_url = std::env::args().nth(1).expect("Please provide a toot URL.");
    let thread = mastodon::fetch_thread(&first_toot_url).await?;
    let blogpost: markdown::Blogpost = thread.into();

    println!("{}", blogpost.markdown);

    // Write to index.md.
    std::fs::write("index.md", blogpost.markdown)?;

    // Download the files, if they don't exist yet, in parallel.
    stream::iter(blogpost.downloads)
        .map(Ok) // Convert to a TryStream.
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
