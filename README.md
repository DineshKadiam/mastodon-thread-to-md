<!--
SPDX-FileCopyrightText: 2026 blinry <mail@blinry.org>

SPDX-License-Identifier: CC-BY-SA-4.0
-->

# mastodon-to-blogpost

Takes the URL to a Mastodon toot, and converts the thread (all children by the same author) to a Markdown blogpost. It also downloads all included media files, and includes their alt text.

## Usage

You'll need Rust's `cargo` command. Then, clone this repo, and run this command in it:

```
cargo run --release https://example.com/@user/12345678987654321
```

This will create an `index.md` file, and download all media files to the current directory, in parallel.

## API limits

Mastodon's default API limit is a maximum thread length of 40. You can increase that to 4096 by providing an access token via an environment variable!

Generate one in the Mastodon settings, under "Development". Give it "read" permissions. Then, run

```bash
export ACCESS_TOKEN="<your access token>"
```

You can also put this line in an `.envrc` file, and activate it using [direnv](https://github.com/direnv/direnv).

## License

GPLv3+. This project is [REUSE](https://reuse.software) compliant.
