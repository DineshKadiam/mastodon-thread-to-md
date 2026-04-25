// SPDX-FileCopyrightText: 2026 blinry <mail@blinry.org>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use clap::Parser;
use color_eyre::{
    Help, Result,
    eyre::{Error, WrapErr, eyre},
};
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

    color_eyre::install()?;

    let args = Args::parse();

    if args.output.exists() {
        return Err(eyre!(
            "{} exists, refusing to overwrite",
            args.output.display()
        ))
        .suggestion("Remove the file manually and try again");
    }

    println!("Fetching thread...");
    let thread = mastodon::fetch_thread(&args.url)
        .await
        .wrap_err("Failed to fetch thread")?;

    println!("Found {} toots.", thread.toots.len());

    let blogpost: markdown::Blogpost = thread.into();

    // Write to Markdown file.
    println!("Writing {}...", args.output.display());
    std::fs::write(&args.output, &blogpost.markdown).wrap_err("Failed to write Markdown file")?;

    // Download the attachments, if they don't exist yet, in parallel.
    download_attachments(&blogpost, &args)
        .await
        .wrap_err("Failed to download attachments")?;

    let duration = start.elapsed();
    println!("Completed in {:.2} s! 🧶🐱", duration.as_secs_f64());

    Ok(())
}

async fn download_attachments(blogpost: &markdown::Blogpost, args: &Args) -> Result<()> {
    let output_clone = args.output.clone();
    let base_dir = output_clone
        .parent()
        .expect("Output path should have a base dir");

    stream::iter(&blogpost.downloads)
        .map(Ok) // Convert to a TryStream.
        .try_for_each_concurrent(8, |Download { url, filename }| async move {
            let target_filename = base_dir.join(filename);
            if !PathBuf::from(&target_filename).exists() {
                println!("Downloading {}...", target_filename.display());
                let bytes = reqwest::get(url).await?.error_for_status()?.bytes().await?;
                std::fs::write(&target_filename, bytes)
                    .wrap_err(format!("Failed to write to {}", target_filename.display()))?;
            }
            Ok::<_, Error>(())
        })
        .await?;

    Ok(())
}
