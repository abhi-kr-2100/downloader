# downloader

[![Crates.io](https://img.shields.io/crates/v/downloader.svg)](https://crates.io/crates/downloader)
[![Docs.rs](https://docs.rs/downloader/badge.svg)](https://docs.rs/downloader)
[![CI](https://github.com/hunger/downloader/workflows/Continuous%20Integration/badge.svg)](https://github.com/hunger/downloader/actions)
[![Coverage Status](https://coveralls.io/repos/github/hunger/downloader/badge.svg?branch=main)](https://coveralls.io/github/hunger/downloader?branch=main)

`downloader` is a crate to help with easily downloading of files from the
internet. It takes a simple simple and straightforward approach using a url
builder and fetcher.

It supports parallel downloads of different files,
validation of downloads via a callback, as well as files mirroring across different
machines.

Callbacks to provide progress information are supported as well.

## Usage

### Installation via Cargo

Add the following line into your `Cargo.toml` file to make `downloader` a
`[dependency]` of your crate:

`downloader = "<VERSION>"`

Alternatively you can run `cargo add downloader`. See crates.io for the latest
version of the package.

### Example

The library is generic over the HTTP backend. You must provide an implementation of the `Backend` trait.

```rust
use downloader::{Downloader, Download, Backend, Response, Result};
use std::path::Path;
use bytes::Bytes;
use async_trait::async_trait;

// Example of a minimal backend implementation (e.g., using reqwest)
#[derive(Clone)]
struct MyBackend(reqwest::Client);

#[async_trait]
impl Backend for MyBackend {
    async fn get(&self, url: &url::Url) -> Result<Box<dyn Response + Send>> {
        let response = self.0.get(url.clone()).send().await
            .map_err(|e| downloader::Error::Backend(e.to_string()))?;
        Ok(Box::new(MyResponse(response)))
    }
}

struct MyResponse(reqwest::Response);

#[async_trait]
impl Response for MyResponse {
    fn content_length(&self) -> Option<u64> { self.0.content_length() }
    fn status(&self) -> http::StatusCode {
        http::StatusCode::from_u16(self.0.status().as_u16()).unwrap()
    }
    async fn chunk(&mut self) -> Result<Option<Bytes>> {
        self.0.chunk().await.map_err(|e| downloader::Error::Backend(e.to_string()))
    }
}

fn main() {
    let mut dl = Downloader::builder()
        .backend(MyBackend(reqwest::Client::new()))
        .download_folder(Path::new("/tmp"))
        .build()
        .unwrap();

    let image = Download::new("https://example.com/example.png");
    let results = dl.download(&[image]).unwrap();

    for result in results {
        match result {
            Ok(summary) => println!("Downloaded: {:?}", summary.file_name),
            Err(e) => println!("Error: {:?}", e),
        }
    }
}
```

### Features

- `tui` feature uses `indicatif` crate to provide a text ui for downloads
- `verify` feature enables (optional) verification of downloads using sha3 hashes

## License

Licensed under the GNU Lesser General Public License, Version 3.0 or later
([LICENSE-LGPLv3](LICENSE-LGPLv3.md) or <https://www.gnu.org/licenses/lgpl.md>)

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, shall be licensed as LGPLv3 or later, without
any additional terms or conditions.

See [CONTRIBUTING.md](CONTRIBUTING.md).
