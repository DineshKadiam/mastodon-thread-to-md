<!--
SPDX-FileCopyrightText: 2026 blinry <mail@blinry.org>

SPDX-License-Identifier: CC-BY-SA-4.0
-->

# mastodon-to-blogpost

A little tool that takes the URL to a Mastodon toot, and converts the thread (all children by the same author) to a Markdown blogpost, in a suitable format for [my homepage](https://github.com/blinry/blinry.org).

## Features

- Collects all toots by the same author, which are not replies to other people.
    - Converts these toots to a single Markdown file.
- Downloads all attached media files.
    - Includes provided alt text for images/videos.
    - Downloads media files in parallel.
    - Only downloads media files if they're not already downloaded.

## API limits

The default API limit is a maximum thread length of 40. You can increase that to 4096 by providing an access token via an environment variable!

Generate one in the Mastodon settings, under "Development". Then, run

```bash
export ACCESS_TOKEN=<your access token>
```

You can also put this line in an `.envrc` file, and activate it using [direnv](https://github.com/direnv/direnv).

## Usage


```
cargo run https://example.com/@user/12345678987654321
```

This will create an `index.md` file, and download all media files to the current directory.
