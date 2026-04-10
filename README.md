<!--
SPDX-FileCopyrightText: 2026 blinry <mail@blinry.org>

SPDX-License-Identifier: CC-BY-SA-4.0
-->

# threadcat 🧶🐱

Takes the URL to a Mastodon toot, and converts the thread (all children by the same author) to a Markdown blogpost. It also downloads all included media files, and their alt texts.

## Installation

You'll need Rust's `cargo` command. Then run:

```
cargo install threadcat
```

## Usage

```
$ threadcat https://example.com/@user/12345678987654321
Fetching thread...
Found 13 toots.
Writing index.md...
Downloading 4fccc6671b254665.jpg...
Downloading 34dbf96666113346.jpg...
Downloading 8599bd70f4d23ba6.jpg...
Completed in 1.09 s! 🧶🐱
```

## API limits

Mastodon's default API limit is a maximum thread length of 40. You can increase that to 4096 by providing an access token via an environment variable!

Generate one in the Mastodon settings, under "Development". Give it "read" permissions. Then, run

```bash
export ACCESS_TOKEN="<your access token>"
```

You can also put this line in an `.envrc` file, and activate it using [direnv](https://github.com/direnv/direnv).

## License

GPLv3+. This project is [REUSE](https://reuse.software) compliant.
