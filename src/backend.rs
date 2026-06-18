// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! The actual download code

use crate::{Download, DownloadSummary, Error, Result, Verification};

use bytes::Bytes;
use futures::stream::{self, StreamExt};
use rand::seq::IndexedRandom;

use std::io::{Seek, SeekFrom, Write};

/// A `Response` of a `Backend`
#[async_trait::async_trait]
pub trait Response {
    /// Get the content length of the response, if available.
    fn content_length(&self) -> Option<u64>;
    /// Get the status code of the response.
    fn status(&self) -> http::StatusCode;
    /// Get the next chunk of data from the response.
    async fn chunk(&mut self) -> Result<Option<Bytes>>;
}

/// A `Backend` to be used for downloading
#[async_trait::async_trait]
pub trait Backend {
    /// Start a GET request to the provided `url`.
    async fn get(&self, url: &url::Url) -> Result<Box<dyn Response + Send>>;
}

fn select_url(urls: &[String]) -> String {
    assert!(!urls.is_empty());
    urls.choose(&mut rand::rng()).unwrap().clone()
}

async fn download_url<B: Backend + ?Sized>(
    backend: &B,
    url_str: &str,
    writer: &mut std::io::BufWriter<std::fs::File>,
    progress: &mut crate::Progress,
    message: &str,
) -> http::StatusCode {
    let Ok(url) = url::Url::parse(url_str) else {
        return http::StatusCode::BAD_REQUEST;
    };

    if let Ok(mut response) = backend.get(&url).await {
        let total = response.content_length();
        let mut current: u64 = 0;
        writer.seek(SeekFrom::Start(current)).unwrap_or(0);

        progress.setup(total, message);

        while let Ok(Some(bytes)) = response.chunk().await {
            if writer.write_all(&bytes).is_err() {
                // TODO: Should we return an error here?
            }

            current += bytes.len() as u64;
            progress.progress(current);
        }

        let result = response.status();
        progress.set_message(&format!("{message} - {result}"));
        result
    } else {
        http::StatusCode::BAD_REQUEST
    }
}

async fn verify_download(
    path: std::path::PathBuf,
    verify_callback: crate::Verify,
    progress: crate::Progress,
    message: &str,
) -> Verification {
    let p = progress.clone();
    let result =
        tokio::task::spawn_blocking(move || verify_callback(path, &move |c: u64| p.progress(c)))
            .await
            .unwrap_or(crate::Verification::NotVerified);
    progress.set_message(&format!(
        "{} - {}",
        message,
        match result {
            Verification::NotVerified => "not verified",
            Verification::Failed => "FAILED",
            Verification::Ok => "Ok",
        }
    ));
    progress.done();
    result
}

async fn download<B: Backend + ?Sized>(
    backend: &B,
    mut download: Download,
    retries: u16,
) -> Result<DownloadSummary> {
    let mut summary = DownloadSummary {
        status: Vec::new(),
        file_name: std::mem::take(&mut download.file_name),
        verified: Verification::NotVerified,
    };

    let mut urls = std::mem::take(&mut download.urls);
    assert!(!urls.is_empty());

    let mut progress = download.progress.expect("This has been set!").clone();
    let mut message = String::new();

    let mut download_successful = false;

    if let Ok(file) = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&summary.file_name)
    {
        let mut writer = std::io::BufWriter::new(file);

        for retry in 1..=retries {
            let url = select_url(&urls);

            message = format!(
                "{} {}/{}",
                &summary
                    .file_name
                    .file_name()
                    .unwrap_or_else(|| std::ffi::OsStr::new("<unknown>"))
                    .to_string_lossy(),
                retry,
                retries,
            );

            let s = download_url(backend, &url, &mut writer, &mut progress, &message).await;

            summary.status.push((url.clone(), s.as_u16()));

            if s.is_server_error() {
                urls = urls
                    .iter()
                    .filter_map(|u| if u == &url { Some(u.clone()) } else { None })
                    .collect();
                if urls.is_empty() {
                    break;
                }
            }

            if s.is_success() {
                download_successful = true;
                break;
            }
        }
    }

    if !download_successful {
        return Err(Error::Download(summary));
    }

    summary.verified = verify_download(
        summary.file_name.clone(),
        std::mem::replace(&mut download.verify_callback, crate::verify::noop()),
        progress.clone(),
        &message,
    )
    .await;
    if summary.verified == Verification::Failed {
        return Err(Error::Verification(summary));
    }

    Ok(summary)
}

/// Run the provided list of `downloads`, using the provided `backend`
pub(crate) fn run<B: Backend + Clone + Send + Sync + 'static>(
    backend: &B,
    downloads: Vec<Download>,
    retries: u16,
    parallel_requests: u16,
) -> Vec<Result<DownloadSummary>> {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let cl = backend.clone();

    let result = rt.spawn(async move {
        stream::iter(downloads)
            .map(|d| {
                let cl = cl.clone();
                async move { download(&cl, d, retries).await }
            })
            .buffer_unordered(parallel_requests as usize)
            .collect::<Vec<Result<DownloadSummary>>>()
            .await
    });

    rt.block_on(result).unwrap()
}

pub(crate) async fn async_run<B: Backend + Clone + Send + Sync + 'static>(
    backend: &B,
    downloads: Vec<Download>,
    retries: u16,
    parallel_requests: u16,
) -> Vec<Result<DownloadSummary>> {
    let cl = backend.clone();

    let result = tokio::spawn(async move {
        stream::iter(downloads)
            .map(|d| {
                let cl = cl.clone();
                async move { download(&cl, d, retries).await }
            })
            .buffer_unordered(parallel_requests as usize)
            .collect::<Vec<Result<DownloadSummary>>>()
            .await
    })
    .await;

    result.unwrap()
}
