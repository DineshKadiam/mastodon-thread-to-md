// SPDX-FileCopyrightText: 2026 blinry <mail@blinry.org>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Fetching data from Mastodon.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use megalodon::entities::status::Status;
use url::Url;

/// A thread of toots, plus some metadata.
#[derive(Debug)]
pub struct Thread {
    pub published: DateTime<Utc>,
    pub toot_url: String,
    pub toots: Vec<Toot>,
}

/// A single toot (with attachments).
#[derive(Clone, Debug)]
pub struct Toot {
    pub html: String,
    pub attachments: Vec<Attachment>,
}

/// A multimedia attachment.
#[derive(Clone, Debug)]
pub struct Attachment {
    pub url: String,
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

pub async fn fetch_thread(first_toot_url: &str) -> anyhow::Result<Thread> {
    let url = Url::parse(first_toot_url).expect("Urls should be valid");
    let client = init_client(&url.origin().ascii_serialization())?;

    let id = url
        .path_segments()
        .unwrap()
        .next_back()
        .unwrap()
        .to_string();
    let first_toot = client.get_status(id.clone()).await?.json();

    // Collect toots of thread.
    let megalodon_toots = collect_thread(&client, first_toot.clone()).await?;

    let toots: Vec<Toot> = megalodon_toots
        .into_iter()
        .map(std::convert::Into::into)
        .collect();

    Ok(Thread {
        published: first_toot.created_at,
        toot_url: first_toot_url.to_string(),
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

impl From<Status> for Toot {
    fn from(status: Status) -> Self {
        let mut attachments = vec![];

        for attachment in &status.media_attachments {
            let attachment = Attachment {
                url: attachment.url.clone(),
                description: attachment.description.clone(),
            };

            attachments.push(attachment);
        }

        Self {
            html: status.content,
            attachments,
        }
    }
}
