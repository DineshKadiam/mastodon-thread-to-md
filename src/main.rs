// SPDX-FileCopyrightText: 2026 blinry <mail@blinry.org>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use clap::Parser;
use futures::{StreamExt, TryStreamExt, stream};
use markdown::Download;
use std::path::PathBuf;
use std::time::Instant;

mod markdown;
mod mastodon;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// URL of the first toot of a Mastodon thread.
    url: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let start = Instant::now();

    let args = Args::parse();

    println!("Fetching thread...");
    let thread = mastodon::fetch_thread(&args.url).await?;

    println!("Found {} toots.", thread.toots.len());

    let blogpost: markdown::Blogpost = thread.into();

    // Write to index.md.
    println!("Writing index.md...");
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

    let duration = start.elapsed();
    println!("Completed in {:.2} s! 🧶🐱", duration.as_secs_f64());

    Ok(())
}
