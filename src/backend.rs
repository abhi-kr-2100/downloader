// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! The actual download code

use crate::{Download, DownloadSummary, Error, Result};

use futures::stream::{self, StreamExt};
use rand::seq::IndexedRandom;

use std::io::{Seek, SeekFrom, Write};

fn select_url(urls: &[String]) -> String {
    assert!(!urls.is_empty());
    urls.choose(&mut rand::rng()).unwrap().clone()
}

async fn download_url(
    client: reqwest::Client,
    url: String,
    writer: &mut std::io::BufWriter<std::fs::File>,
    progress: &mut crate::Progress,
    message: &str,
) -> u16 {
    if let Ok(mut response) = client.get(&url).send().await {
        let total = response.content_length();
        let mut current: u64 = 0;
        writer.seek(SeekFrom::Start(current)).unwrap_or(0);

        progress.setup(total, message);

        while let Some(bytes) = response.chunk().await.unwrap_or(None) {
            if writer.write_all(&bytes).is_err() {}

            current += bytes.len() as u64;
            progress.progress(current);
        }

        let result = response.status().as_u16();
        progress.set_message(&format!("{message} - {result}"));
        result
    } else {
        reqwest::StatusCode::BAD_REQUEST.as_u16()
    }
}

async fn download(
    client: reqwest::Client,
    mut download: Download,
    retries: u16,
) -> Result<DownloadSummary> {
    let mut summary = DownloadSummary {
        status: Vec::new(),
        file_name: std::mem::take(&mut download.file_name),
    };

    let mut urls = std::mem::take(&mut download.urls);
    assert!(!urls.is_empty());

    let mut progress = download.progress.expect("This has been set!").clone();

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

            let message = format!(
                "{} {}/{}",
                &summary
                    .file_name
                    .file_name()
                    .unwrap_or_else(|| std::ffi::OsStr::new("<unknown>"))
                    .to_string_lossy(),
                retry,
                retries,
            );

            let s = reqwest::StatusCode::from_u16(
                download_url(
                    client.clone(),
                    url.clone(),
                    &mut writer,
                    &mut progress,
                    &message,
                )
                .await,
            )
            .unwrap_or(reqwest::StatusCode::BAD_REQUEST);

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

    progress.done();
    Ok(summary)
}

/// Run the provided list of `downloads`, using the provided `client`
pub(crate) fn run(
    client: &mut reqwest::Client,
    downloads: Vec<Download>,
    retries: u16,
    parallel_requests: u16,
) -> Vec<Result<DownloadSummary>> {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let cl = client.clone();

    let result = rt.spawn(async move {
        stream::iter(downloads)
            .map(move |d| download(cl.clone(), d, retries))
            .buffer_unordered(parallel_requests as usize)
            .collect::<Vec<Result<DownloadSummary>>>()
            .await
    });

    rt.block_on(result).unwrap()
}

pub(crate) async fn async_run(
    client: &mut reqwest::Client,
    downloads: Vec<Download>,
    retries: u16,
    parallel_requests: u16,
) -> Vec<Result<DownloadSummary>> {
    let cl = client.clone();

    let result = tokio::spawn(async move {
        stream::iter(downloads)
            .map(move |d| download(cl.clone(), d, retries))
            .buffer_unordered(parallel_requests as usize)
            .collect::<Vec<Result<DownloadSummary>>>()
            .await
    })
    .await;

    result.unwrap()
}
