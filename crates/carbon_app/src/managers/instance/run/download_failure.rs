//! What the player is told when a batch of downloads fails: what could not
//! be downloaded, why, and what to do about it. The tile shows only the
//! outermost sentence of a failure, so the cause is said there, not left to
//! the log. The download's own error stays behind it for the log.

use carbon_net::DownloadError;
use std::path::Path;

const TRY_AGAIN: &str = "Press Play to try again.";

/// The failure of a batch `subject` describes, such as "The game's files
/// could not be downloaded".
pub(super) fn explain(subject: &str, error: DownloadError) -> anyhow::Error {
    let message = format!("{subject}: {}", reason(&error));
    anyhow::Error::new(error).context(message)
}

fn file_name(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
}

/// A request that got no answer. `host` is where it went, when known.
fn unanswered(host: Option<&str>, timed_out: bool) -> String {
    let server = match host {
        Some(host) => format!("the download server ({host})"),
        None => "the download server".to_owned(),
    };
    let what = if timed_out {
        "did not answer in time"
    } else {
        "could not be reached"
    };
    format!("{server} {what}. Check your internet connection, then press Play again.")
}

fn request_failure(error: &reqwest::Error) -> String {
    unanswered(
        error.url().and_then(|url| url.host_str()),
        error.is_timeout(),
    )
}

fn reason(error: &DownloadError) -> String {
    match error {
        DownloadError::NetworkError(error) => request_failure(error),
        DownloadError::NetworkError2(reqwest_middleware::Error::Reqwest(error)) => {
            request_failure(error)
        }
        DownloadError::NetworkError2(reqwest_middleware::Error::Middleware(_)) => {
            unanswered(None, false)
        }
        DownloadError::Non200StatusCode(file, status) => format!(
            "the server answered {status} for {}. {TRY_AGAIN}",
            file_name(&file.path)
        ),
        DownloadError::ChecksumMismatch { path, .. } => {
            format!("{} arrived damaged. {TRY_AGAIN}", file_name(path))
        }
        DownloadError::SizeMismatch { .. } => format!("a file arrived damaged. {TRY_AGAIN}"),
        DownloadError::GenericDownload(what) => format!("{what}. {TRY_AGAIN}"),
        // The system's own words: a full disk, a folder that can't be
        // written.
        DownloadError::IOError(error) => format!("{error}."),
        DownloadError::CannotMoveFile(error, _, destination) => format!(
            "{} could not be put in place: {error}.",
            file_name(destination)
        ),
        DownloadError::OwnedPartialCleanup { original, .. } => reason(original),
        other => format!("{other}."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use carbon_net::Downloadable;

    fn told(error: DownloadError) -> String {
        explain("The game's files could not be downloaded", error).to_string()
    }

    /// A failed download said only "Failed to download
    /// instance files for instance 12". Each cause now has its own sentence.
    #[test]
    fn each_cause_of_a_failed_download_is_said_with_what_to_do() {
        assert_eq!(
            told(DownloadError::Non200StatusCode(
                Downloadable::new(
                    "https://example.invalid/client.jar",
                    "/data/libraries/net/minecraft/client-1.21.jar"
                ),
                404
            )),
            "The game's files could not be downloaded: the server answered 404 for client-1.21.jar. Press Play to try again."
        );
        assert_eq!(
            told(DownloadError::ChecksumMismatch {
                expected: "aa".into(),
                actual: "bb".into(),
                url: "https://example.invalid/lwjgl.jar".into(),
                path: "/data/libraries/org/lwjgl/lwjgl-3.3.3.jar".into(),
            }),
            "The game's files could not be downloaded: lwjgl-3.3.3.jar arrived damaged. Press Play to try again."
        );
        assert_eq!(
            told(DownloadError::SizeMismatch {
                expected: 10,
                actual: 4
            }),
            "The game's files could not be downloaded: a file arrived damaged. Press Play to try again."
        );
        assert_eq!(
            told(DownloadError::GenericDownload(
                "download stalled: no data received before the read timeout".into()
            )),
            "The game's files could not be downloaded: download stalled: no data received before the read timeout. Press Play to try again."
        );
        let full = std::io::Error::from_raw_os_error(28);
        assert_eq!(
            told(DownloadError::IOError(std::io::Error::from_raw_os_error(
                28
            ))),
            format!("The game's files could not be downloaded: {full}.")
        );
        assert_eq!(
            told(DownloadError::CannotMoveFile(
                std::io::Error::from_raw_os_error(13),
                "/data/libraries/a.jar.part".into(),
                "/data/libraries/a.jar".into()
            )),
            format!(
                "The game's files could not be downloaded: a.jar could not be put in place: {}.",
                std::io::Error::from_raw_os_error(13)
            )
        );
        // A failure that also could not clean up after itself is told by
        // what went wrong first.
        assert_eq!(
            told(DownloadError::OwnedPartialCleanup {
                original: Box::new(DownloadError::SizeMismatch {
                    expected: 10,
                    actual: 4
                }),
                path: "/data/libraries/a.jar.part".into(),
                cleanup: std::io::Error::from_raw_os_error(13),
            }),
            "The game's files could not be downloaded: a file arrived damaged. Press Play to try again."
        );
        // Anything else keeps the download's own words after the same opening.
        assert_eq!(
            told(DownloadError::InvalidRange),
            "The game's files could not be downloaded: Invalid partial response range."
        );
    }

    /// No connection: the sentence names the server that could not be
    /// reached, and the request's own error stays behind it for the log.
    #[tokio::test]
    async fn a_server_that_cannot_be_reached_is_named() {
        // Nothing listens on port 1.
        let refused = reqwest::Client::new()
            .get("http://127.0.0.1:1/client.jar")
            .send()
            .await
            .expect_err("nothing listens there");
        let error = explain(
            "The modpack's files could not be downloaded",
            DownloadError::NetworkError(refused),
        );
        assert_eq!(
            error.to_string(),
            "The modpack's files could not be downloaded: the download server (127.0.0.1) could not be reached. Check your internet connection, then press Play again."
        );
        assert!(
            error.root_cause().downcast_ref::<DownloadError>().is_some()
                || format!("{error:#}").contains("Failed to make network request"),
            "{error:#}"
        );
    }

    #[test]
    fn a_request_that_got_no_answer_says_whether_it_timed_out() {
        assert_eq!(
            unanswered(Some("meta.example"), true),
            "the download server (meta.example) did not answer in time. Check your internet connection, then press Play again."
        );
        assert_eq!(
            unanswered(None, false),
            "the download server could not be reached. Check your internet connection, then press Play again."
        );
    }
}
