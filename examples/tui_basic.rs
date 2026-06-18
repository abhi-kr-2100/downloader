// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>
// Copyright (C) 2021 Phoenix IR <ayitsmephoenix@firemail.cc>

// Setup warnings/errors:
#![forbid(unsafe_code)]
#![deny(bare_trait_objects, unused_doc_comments, unused_import_braces)]
// Clippy:
#![warn(clippy::all, clippy::nursery, clippy::pedantic)]
#![allow(clippy::non_ascii_literal)]

use downloader::Downloader;
use std::env::temp_dir;

/// A fake response that returns some dummy data
struct FakeResponse {
    data: Vec<u8>,
    read: bool,
}

#[async_trait::async_trait]
impl downloader::Response for FakeResponse {
    fn content_length(&self) -> Option<u64> {
        Some(self.data.len() as u64)
    }

    fn status(&self) -> http::StatusCode {
        http::StatusCode::OK
    }

    async fn chunk(&mut self) -> downloader::Result<Option<bytes::Bytes>> {
        if self.read {
            Ok(None)
        } else {
            self.read = true;
            Ok(Some(bytes::Bytes::copy_from_slice(&self.data)))
        }
    }
}

#[derive(Clone)]
struct FakeBackend;

#[async_trait::async_trait]
impl downloader::Backend for FakeBackend {
    async fn get(&self, _url: &url::Url) -> downloader::Result<Box<dyn downloader::Response + Send>> {
        Ok(Box::new(FakeResponse {
            data: b"<html><body>Hello World</body></html>".to_vec(),
            read: false,
        }))
    }
}

// Run example with: cargo run --example tui_basic --features tui
fn main() {
    let backend = FakeBackend;

    let mut downloader = Downloader::builder()
        .backend(backend)
        .download_folder(&temp_dir())
        .parallel_requests(1)
        .build()
        .unwrap();

    // Download with an explicit filename
    let dl = downloader::Download::new("https://example.org/")
        .file_name(std::path::Path::new("example.html"));

    // Download with an inferred filename
    let dl2 = downloader::Download::new(
        "https://example.com/test.iso",
    );

    let result = downloader.download(&[dl, dl2]).unwrap();

    for r in result {
        match r {
            Err(e) => print!("Error occurred! {e}"),
            Ok(s) => print!("Success: {}", &s),
        };
    }
}
