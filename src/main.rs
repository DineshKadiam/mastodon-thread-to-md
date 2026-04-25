// SPDX-FileCopyrightText: 2026 blinry <mail@blinry.org>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Result;
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

    /// File that the Markdown content should be written to.
    #[arg(short, long, default_value = "index.md")]
    output: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    let start = Instant::now();

    let args = Args::parse();

    assert!(
        !args.output.exists(),
        "{} exists, please remove it before running threadcat",
        args.output.display()
    );

    println!("Fetching thread...");
    let thread = mastodon::fetch_thread(&args.url).await?;

    println!("Found {} toots.", thread.toots.len());

    let blogpost: markdown::Blogpost = thread.into();

    // Write to Markdown file.
    println!("Writing {}...", args.output.display());
    std::fs::write(&args.output, &blogpost.markdown)?;

    // Download the attachments, if they don't exist yet, in parallel.
    download_attachments(&blogpost, &args).await?;

    let duration = start.elapsed();
    println!("Completed in {:.2} s! 🧶🐱", duration.as_secs_f64());

    Ok(())
}

async fn download_attachments(blogpost: &markdown::Blogpost, args: &Args) -> Result<()> {
    let output_clone = args.output.clone();
    let base_dir = output_clone
        .parent()
        .expect("output path should have a base dir");

    stream::iter(&blogpost.downloads)
        .map(Ok) // Convert to a TryStream.
        .try_for_each_concurrent(8, |Download { url, filename }| async move {
            let target_filename = base_dir.join(filename);
            if !PathBuf::from(&target_filename).exists() {
                println!("Downloading {}...", target_filename.display());
                let bytes = reqwest::get(url).await?.bytes().await?;
                std::fs::write(&target_filename, bytes)?;
            }
            Ok::<_, anyhow::Error>(())
        })
        .await?;

    Ok(())
}
