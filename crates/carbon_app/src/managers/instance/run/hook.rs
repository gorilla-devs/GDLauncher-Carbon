//! The player's own scripts around a game: one before it starts and one
//! after it ends. Both are run the same way, by [`run`]: in the instance's
//! game folder, as a process the runtime tracks, and for a limited time.

use crate::managers::AppInner;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookKind {
    PreLaunch,
    PostExit,
}

impl HookKind {
    fn label(self) -> &'static str {
        match self {
            Self::PreLaunch => "Pre-launch hook",
            Self::PostExit => "Post-exit hook",
        }
    }
}

/// Why a hook did not run to a successful end.
#[derive(Debug)]
pub enum HookFailure {
    /// The command is not one a shell would accept, such as an open quote.
    Unparsable,
    Empty,
    /// It ran for the whole time it is given.
    TimedOut(Duration),
    /// The launcher's own shutdown stopped it.
    Stopped(std::io::Error),
    NotStarted(std::io::Error),
    /// It ended with an error; `stderr` is what it printed there.
    Failed {
        status: std::process::ExitStatus,
        stderr: String,
    },
}

#[derive(Debug)]
pub struct HookError {
    pub kind: HookKind,
    pub failure: HookFailure,
}

impl std::fmt::Display for HookError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hook = self.kind.label();
        match &self.failure {
            HookFailure::Unparsable => write!(f, "{hook} could not be read as a command"),
            HookFailure::Empty => write!(f, "{hook} is empty"),
            HookFailure::TimedOut(limit) => write!(f, "{hook} did not finish within {limit:?}"),
            HookFailure::Stopped(error) => {
                write!(f, "{hook} was stopped before it finished: {error:?}")
            }
            HookFailure::NotStarted(error) => write!(f, "{hook} failed to start: {error:?}"),
            HookFailure::Failed { status, stderr } => {
                write!(f, "{hook} failed with status: {status:?} \n{stderr}")
            }
        }
    }
}

impl std::error::Error for HookError {}

/// Runs the hook `command` in `folder`, for `limit` at most, and returns
/// what it printed. A quit or restart of the launcher lets a hook that is
/// already running finish.
pub async fn run(
    app: &Arc<AppInner>,
    kind: HookKind,
    command: &str,
    folder: &Path,
    limit: Duration,
) -> Result<String, HookError> {
    let failed = |failure| HookError { kind, failure };
    let mut words = super::split_hook_command(command)
        .ok_or_else(|| failed(HookFailure::Unparsable))?
        .into_iter();
    let program = words.next().ok_or_else(|| failed(HookFailure::Empty))?;
    let output = tokio::time::timeout(
        limit,
        crate::managers::runtime_lifecycle::tracked_output_reserved(
            &app.runtime_lifecycle,
            tokio::process::Command::new(program)
                .args(words)
                .current_dir(folder)
                .kill_on_drop(true),
            crate::managers::runtime_lifecycle::ActivityPolicy::Hook,
            None,
        ),
    )
    .await
    .map_err(|_| failed(HookFailure::TimedOut(limit)))?
    .map_err(|error| {
        failed(if error.kind() == std::io::ErrorKind::Interrupted {
            HookFailure::Stopped(error)
        } else {
            HookFailure::NotStarted(error)
        })
    })?;
    if !output.status.success() {
        return Err(failed(HookFailure::Failed {
            status: output.status,
            stderr: String::from_utf8(output.stderr).unwrap_or_default(),
        }));
    }
    Ok(String::from_utf8(output.stdout).unwrap_or_default())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    const LIMIT: Duration = Duration::from_secs(20);

    #[tokio::test]
    async fn a_hook_runs_in_the_folder_given_and_returns_what_it_printed() {
        let app = crate::setup_managers_for_test().await;
        let folder = tempfile::tempdir().unwrap();
        let printed = run(
            &app,
            HookKind::PostExit,
            "sh -c 'pwd; touch ran-here'",
            folder.path(),
            LIMIT,
        )
        .await
        .unwrap();
        assert_eq!(
            std::fs::canonicalize(printed.trim()).unwrap(),
            std::fs::canonicalize(folder.path()).unwrap()
        );
        assert!(folder.path().join("ran-here").is_file());
    }

    #[tokio::test]
    async fn each_way_a_hook_fails_is_told_apart_and_names_the_hook() {
        let app = crate::setup_managers_for_test().await;
        let folder = tempfile::tempdir().unwrap();
        let failure = |kind, command: &'static str, limit| {
            let (app, folder) = (app.clone(), folder.path().to_owned());
            async move {
                run(&app, kind, command, &folder, limit)
                    .await
                    .expect_err("the hook fails")
            }
        };

        let error = failure(HookKind::PreLaunch, "sh -c 'echo why >&2; exit 3'", LIMIT).await;
        assert!(
            matches!(&error.failure, HookFailure::Failed { status, stderr } if status.code() == Some(3) && stderr == "why\n"),
            "{error}"
        );
        assert!(
            error
                .to_string()
                .starts_with("Pre-launch hook failed with status: "),
            "{error}"
        );
        assert!(error.to_string().ends_with("\nwhy\n"), "{error}");

        let error = failure(HookKind::PostExit, "sh -c 'unterminated", LIMIT).await;
        assert!(matches!(error.failure, HookFailure::Unparsable), "{error}");
        assert_eq!(
            error.to_string(),
            "Post-exit hook could not be read as a command"
        );

        let error = failure(HookKind::PostExit, "", LIMIT).await;
        assert!(matches!(error.failure, HookFailure::Empty), "{error}");

        let error = failure(HookKind::PostExit, "/no/such/program --flag", LIMIT).await;
        assert!(
            matches!(error.failure, HookFailure::NotStarted(_)),
            "{error}"
        );
        assert!(
            error
                .to_string()
                .starts_with("Post-exit hook failed to start"),
            "{error}"
        );

        let error = failure(HookKind::PostExit, "sleep 30", Duration::from_millis(200)).await;
        assert!(matches!(error.failure, HookFailure::TimedOut(_)), "{error}");
        assert_eq!(
            error.to_string(),
            "Post-exit hook did not finish within 200ms"
        );
    }
}
