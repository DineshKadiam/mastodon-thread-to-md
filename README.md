<!--
SPDX-FileCopyrightText: 2026 blinry <mail@blinry.org>

SPDX-License-Identifier: CC-BY-SA-4.0
-->

# mastodon-to-blogpost

A little tool that takes the URL to a Mastodon toot, and converts the thread (all children by the same author) to a Markdown blogpost, in a suitable format for [my homepage](https://github.com/blinry/blinry.org). It also downloads all attached media files.

## Example run

```
cargo run https://example.com/@user/12345678987654321
```

This will create an `index.md` file, and download all media files to the current directory.
