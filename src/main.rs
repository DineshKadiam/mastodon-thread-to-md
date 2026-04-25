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

    /// File that the Markdown content should be written to.
    #[arg(short, long, default_value = "index.md")]
    output: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let start = Instant::now();

    let args = Args::parse();

    println!("Fetching thread...");
    let thread = mastodon::fetch_thread(&args.url).await?;

    println!("Found {} toots.", thread.toots.len());

    let blogpost: markdown::Blogpost = thread.into();

    let output_clone = args.output.clone();
    let base_dir = output_clone
        .parent()
        .expect("output path should have a base dir");

    // Write to Markdown file.
    println!("Writing {}...", args.output.display());
    std::fs::write(args.output, blogpost.markdown)?;

    // Download the files, if they don't exist yet, in parallel.
    stream::iter(blogpost.downloads)
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

    let duration = start.elapsed();
    println!("Completed in {:.2} s! 🧶🐱", duration.as_secs_f64());

    Ok(())
}
