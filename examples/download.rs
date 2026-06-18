// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

// Setup warnings/errors:
#![forbid(unsafe_code)]
#![deny(bare_trait_objects, unused_doc_comments, unused_import_braces)]
// Clippy:
#![warn(clippy::all, clippy::nursery, clippy::pedantic)]
#![allow(clippy::non_ascii_literal)]

use std::env::temp_dir;

use downloader::Downloader;

// Define a custom progress reporter:
struct SimpleReporterPrivate {
    last_update: std::time::Instant,
    max_progress: Option<u64>,
    message: String,
}
struct SimpleReporter {
    private: std::sync::Mutex<Option<SimpleReporterPrivate>>,
}

impl SimpleReporter {
    #[cfg(not(feature = "tui"))]
    fn create() -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self {
            private: std::sync::Mutex::new(None),
        })
    }
}

impl downloader::progress::Reporter for SimpleReporter {
    fn setup(&self, max_progress: Option<u64>, message: &str) {
        let private = SimpleReporterPrivate {
            last_update: std::time::Instant::now(),
            max_progress,
            message: message.to_owned(),
        };

        let mut guard = self.private.lock().unwrap();
        *guard = Some(private);
    }

    fn progress(&self, current: u64) {
        if let Some(p) = self.private.lock().unwrap().as_mut() {
            let max_bytes = p
                .max_progress
                .map_or_else(|| "{unknown}".to_owned(), |bytes| format!("{bytes:?}"));
            if p.last_update.elapsed().as_millis() >= 1000 {
                println!(
                    "test file: {} of {} bytes. [{}]",
                    current, max_bytes, p.message
                );
                p.last_update = std::time::Instant::now();
            }
        }
    }

    fn set_message(&self, message: &str) {
        println!("test file: Message changed to: {message}");
    }

    fn done(&self) {
        _ = self.private.lock().unwrap().take();
        println!("test file: [DONE]");
    }
}

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
            data: vec![0; 1024 * 1024], // 1MB of zeros
            read: false,
        }))
    }
}

fn main() {
    let backend = FakeBackend;

    let mut downloader = Downloader::builder()
        .backend(backend)
        .download_folder(&temp_dir())
        .parallel_requests(1)
        .build()
        .unwrap();

    let dl = downloader::Download::new("https://example.com/fake.bin");

    #[cfg(not(feature = "tui"))]
    let dl = dl.progress(SimpleReporter::create());

    #[cfg(feature = "verify")]
    let dl = {
        use downloader::verify;
        fn decode_hex(s: &str) -> Result<Vec<u8>, std::num::ParseIntError> {
            (0..s.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
                .collect()
        }
        // This will probably fail verification as it's just zeros
        dl.verify(verify::with_digest::<sha3::Sha3_256>(
            decode_hex("2197e485d463ac2b868e87f0d4547b4223ff5220a0694af2593cbe7c796f7fd6").unwrap(),
        ))
    };

    let result = downloader.download(&[dl]).unwrap();

    for r in result {
        match r {
            Err(e) => println!("Error: {e}"),
            Ok(s) => println!("Success: {}", &s),
        };
    }
}
