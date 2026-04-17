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
#[derive(Debug)]
pub struct Toot {
    pub html: String,
    pub attachments: Vec<Attachment>,
}

/// A multimedia attachment.
#[derive(Debug)]
pub struct Attachment {
    pub url: String,
    pub description: Option<String>,
}

pub async fn fetch_thread(first_toot_url: &str) -> anyhow::Result<Thread> {
    let url = Url::parse(first_toot_url).expect("Urls should be valid");
    let client = Client::new(&url.origin().ascii_serialization())?;

    let id = url
        .path_segments()
        .unwrap()
        .next_back()
        .unwrap()
        .to_string();
    let first_toot = client.megalodon.get_status(id).await?.json();

    let megalodon_toots = client.toots_in_thread(first_toot.clone()).await?;

    // Convert from Megalodon Status to (our) Toot format.
    let toots = megalodon_toots.into_iter().map(Into::into).collect();

    Ok(Thread {
        published: first_toot.created_at,
        toot_url: first_toot_url.to_string(),
        toots,
    })
}

struct Client {
    megalodon: Box<dyn megalodon::Megalodon + Send + Sync>,
}

impl Client {
    fn new(domain: &str) -> Result<Self> {
        let access_token = std::env::var("MASTODON_ACCESS_TOKEN").ok();

        if access_token.is_none() {
            println!(
                "Note: Thread might be cut off due to Mastodon's API limits. Provide an MASTODON_ACCESS_TOKEN via environment variable to make authenticated requests. See the README of the project for details."
            );
        }

        let megalodon = megalodon::generator(
            megalodon::SNS::Mastodon,
            String::from(domain),
            access_token,
            None,
        )
        .context("Failed to initialize Mastodon API client");

        Ok(Self {
            megalodon: megalodon?,
        })
    }

    async fn toots_in_thread(&self, first_toot: Status) -> Result<Vec<Status>> {
        let account = first_toot.account.clone();
        let context = self
            .megalodon
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
            if let Some(ref reply_to_account_id) = toot.in_reply_to_account_id
                && reply_to_account_id != &account.id
            {
                continue;
            }
            toots.push(toot);
        }

        Ok(toots)
    }
}

impl From<Status> for Toot {
    fn from(status: Status) -> Self {
        let attachments = status
            .media_attachments
            .into_iter()
            .map(Into::into)
            .collect();

        Self {
            html: status.content,
            attachments,
        }
    }
}

impl From<megalodon::entities::Attachment> for Attachment {
    fn from(attachment: megalodon::entities::Attachment) -> Self {
        Self {
            url: attachment.url.clone(),
            description: attachment.description,
        }
    }
}
