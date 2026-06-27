# downloader

`downloader` is a crate to help with easily downloading of files from the
internet. It takes a simple simple and straightforward approach using a url
builder and fetcher.

It supports system proxy configuration, parallel downloads of different files,
as well as files mirroring across different machines.

Callbacks to provide progress information are supported as well.

## Usage

### Example

```rust
use downloader::downloader::Builder;
use downloader::Download;
use std::path::Path;
use std::time::Duration;

fn main() {
    let image = Download::new("https://example.com/example.png");
    // other downloads...
    // image.urls.push("https://example.com/example2.png");

    let mut dl = Builder::default()
        .connect_timeout(Duration::from_secs(4))
        .download_folder(Path::new("../res")) // or any arbitrary path
        .parallel_requests(8)
        .build()
        .unwrap();

    let response = dl.download(&[image]).unwrap(); // other error handling

    response.iter().for_each(|v| match v {
        Ok(v) => println!("Downloaded: {:?}", v),
        Err(e) => println!("Error: {:?}", e),
    })
}
```

## License

Licensed under the GNU Lesser General Public License, Version 3.0 or later
([LICENSE-LGPLv3](LICENSE-LGPLv3.md) or <https://www.gnu.org/licenses/lgpl.md>)
