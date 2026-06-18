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

struct ReqwestResponse(reqwest::Response);

#[async_trait::async_trait]
impl downloader::Response for ReqwestResponse {
    fn content_length(&self) -> Option<u64> {
        self.0.content_length()
    }

    fn status(&self) -> http::StatusCode {
        http::StatusCode::from_u16(self.0.status().as_u16()).unwrap()
    }

    async fn chunk(&mut self) -> downloader::Result<Option<bytes::Bytes>> {
        self.0
            .chunk()
            .await
            .map_err(|e| downloader::Error::Backend(e.to_string()))
    }
}

#[derive(Clone)]
struct ReqwestBackend(reqwest::Client);

#[async_trait::async_trait]
impl downloader::Backend for ReqwestBackend {
    async fn get(&self, url: &url::Url) -> downloader::Result<Box<dyn downloader::Response + Send>> {
        let response = self
            .0
            .get(url.clone())
            .send()
            .await
            .map_err(|e| downloader::Error::Backend(e.to_string()))?;
        Ok(Box::new(ReqwestResponse(response)))
    }
}

// Run example with: cargo run --example tui_basic --features tui
fn main() {
    let client = reqwest::Client::new();
    let backend = ReqwestBackend(client);

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
        "https://cdimage.debian.org/debian-cd/12.8.0/i386/iso-cd/debian-12.8.0-i386-netinst.iso",
    );

    let result = downloader.download(&[dl, dl2]).unwrap();

    for r in result {
        match r {
            Err(e) => print!("Error occurred! {e}"),
            Ok(s) => print!("Success: {}", &s),
        };
    }
}
