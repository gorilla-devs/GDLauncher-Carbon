//! The player's own scripts around a game: one before it starts and one
//! after it ends. Both are run the same way, by [`run`]: in the instance's
//! game folder, as a process the runtime tracks, and for a limited time.

use crate::api::keys::instance::GET_HOOK_RUNS;
use crate::domain::instance::InstanceId;
use crate::managers::instance::InstanceManager;
use crate::managers::{AppInner, ManagerRef};
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    /// It ended with an error, having printed `stdout` and `stderr`.
    Failed {
        status: std::process::ExitStatus,
        stdout: String,
        stderr: String,
    },
}

/// The last line with anything on it.
fn last_line(text: &str) -> Option<&str> {
    text.lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty())
}

impl HookFailure {
    /// One line for the player on why the hook failed: the last thing the
    /// script printed, or what stopped it from running.
    pub fn summary(&self) -> String {
        match self {
            Self::Unparsable => "The command could not be read. Check its quotes.".into(),
            Self::Empty => "The command is empty.".into(),
            Self::TimedOut(limit) => {
                format!("It did not finish within {} minutes.", limit.as_secs() / 60)
            }
            Self::Stopped(_) => "GDLauncher stopped it while closing.".into(),
            Self::NotStarted(error) => error.to_string(),
            Self::Failed {
                status,
                stdout,
                stderr,
            } => last_line(stderr)
                .or_else(|| last_line(stdout))
                .map(str::to_owned)
                .unwrap_or_else(|| match status.code() {
                    Some(code) => format!("Exit code {code}."),
                    None => "It was ended before it finished.".into(),
                }),
        }
    }
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
            HookFailure::Failed { status, stderr, .. } => {
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
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// How many hook runs wait for the app at most; the oldest go first.
const KEPT: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookRunState {
    Running,
    Finished,
    /// With [`HookFailure::summary`].
    Failed {
        summary: String,
    },
}

/// One run of a hook around a game, for the app to tell the player of: when
/// it starts, and how it ends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookRun {
    pub id: u32,
    pub instance_id: InstanceId,
    /// The instance's name when the hook started.
    pub name: String,
    pub kind: HookKind,
    pub state: HookRunState,
}

/// The hook runs the app has not finished telling: those still running, and
/// those that ended and wait to be acknowledged.
#[derive(Debug, Default)]
pub struct HookRuns {
    next: AtomicU32,
    runs: Mutex<VecDeque<HookRun>>,
}

impl HookRuns {
    fn begin(&self, instance_id: InstanceId, name: String, kind: HookKind) -> u32 {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let mut runs = self.runs.lock().unwrap();
        // Room is made from runs that ended, never from one still running.
        if runs.len() >= KEPT {
            if let Some(ended) = runs
                .iter()
                .position(|run| run.state != HookRunState::Running)
            {
                runs.remove(ended);
            }
        }
        runs.push_back(HookRun {
            id,
            instance_id,
            name,
            kind,
            state: HookRunState::Running,
        });
        id
    }

    fn end(&self, id: u32, state: HookRunState) {
        if let Some(run) = self
            .runs
            .lock()
            .unwrap()
            .iter_mut()
            .find(|run| run.id == id)
        {
            run.state = state;
        }
    }

    pub fn all(&self) -> Vec<HookRun> {
        self.runs.lock().unwrap().iter().cloned().collect()
    }

    /// Forgets a run that ended. One still running stays.
    pub fn acknowledge(&self, id: u32) {
        self.runs
            .lock()
            .unwrap()
            .retain(|run| run.id != id || run.state == HookRunState::Running);
    }
}

/// What a test of a hook did.
#[derive(Debug)]
pub struct HookTest {
    pub outcome: HookTestOutcome,
    pub millis: u64,
    /// The end of what it printed, when it ran.
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum HookTestOutcome {
    Finished,
    /// It ended with an error, with this exit code when it has one.
    Failed {
        code: Option<i32>,
    },
    /// It never ran, for the reason in [`HookFailure::summary`].
    NotRun {
        summary: String,
    },
    TimedOut,
    /// The player stopped it.
    Stopped,
}

/// How much of each of a test's outputs is shown: its end.
const SHOWN_OUTPUT: usize = 8 * 1024;

fn tail(text: String) -> String {
    if text.len() <= SHOWN_OUTPUT {
        return text;
    }
    let mut start = text.len() - SHOWN_OUTPUT;
    while !text.is_char_boundary(start) {
        start += 1;
    }
    // From a whole line on.
    match text[start..].find('\n') {
        Some(line) => text[start + line + 1..].to_owned(),
        None => text[start..].to_owned(),
    }
}

/// The tests of hooks now running, each with what stops it.
pub type HookTests = Mutex<HashMap<(InstanceId, HookKind), CancellationToken>>;

impl ManagerRef<'_, InstanceManager> {
    /// Runs a hook around a game of `instance_id` and tells the player of
    /// it: that it started, and how it ended.
    pub(super) async fn run_hook_told(
        self,
        instance_id: InstanceId,
        name: String,
        kind: HookKind,
        command: &str,
        folder: &Path,
        limit: Duration,
    ) -> Result<String, HookError> {
        let id = self.hook_runs.begin(instance_id, name, kind);
        self.app.invalidate(GET_HOOK_RUNS, None);
        let ran = run(self.app, kind, command, folder, limit).await;
        self.hook_runs.end(
            id,
            match &ran {
                Ok(_) => HookRunState::Finished,
                Err(error) => HookRunState::Failed {
                    summary: error.failure.summary(),
                },
            },
        );
        self.app.invalidate(GET_HOOK_RUNS, None);
        ran
    }

    /// Runs `command` now as the instance's hook would run, in its game
    /// folder and for the same time at most, without starting the game, and
    /// returns what it did. For trying a hook while writing it: the command
    /// need not be saved. [`stop_hook_test`](Self::stop_hook_test) ends it.
    pub async fn test_hook(
        self,
        instance_id: InstanceId,
        kind: HookKind,
        command: String,
    ) -> anyhow::Result<HookTest> {
        let (shortpath, _) = self.current_config(instance_id).await?;
        let folder = self
            .app
            .settings_manager()
            .runtime_path
            .get_instances()
            .get_instance_path(&shortpath)
            .get_data_path();
        tokio::fs::create_dir_all(&folder).await?;
        let stop = CancellationToken::new();
        {
            let mut tests = self.hook_tests.lock().unwrap();
            if tests.contains_key(&(instance_id, kind)) {
                anyhow::bail!("A test of this hook is already running.");
            }
            tests.insert((instance_id, kind), stop.clone());
        }
        let started = std::time::Instant::now();
        let ran = tokio::select! {
            ran = run(self.app, kind, &command, &folder, super::HOOK_TIMEOUT) => Some(ran),
            // Dropping the run ends the script.
            _ = stop.cancelled() => None,
        };
        self.hook_tests.lock().unwrap().remove(&(instance_id, kind));
        let millis = started.elapsed().as_millis() as u64;
        let (outcome, stdout, stderr) = match ran {
            None => (HookTestOutcome::Stopped, String::new(), String::new()),
            Some(Ok(stdout)) => (HookTestOutcome::Finished, stdout, String::new()),
            Some(Err(error)) => match error.failure {
                HookFailure::Failed {
                    status,
                    stdout,
                    stderr,
                } => (
                    HookTestOutcome::Failed {
                        code: status.code(),
                    },
                    stdout,
                    stderr,
                ),
                HookFailure::TimedOut(_) => {
                    (HookTestOutcome::TimedOut, String::new(), String::new())
                }
                failure => (
                    HookTestOutcome::NotRun {
                        summary: failure.summary(),
                    },
                    String::new(),
                    String::new(),
                ),
            },
        };
        Ok(HookTest {
            outcome,
            millis,
            stdout: tail(stdout),
            stderr: tail(stderr),
        })
    }

    /// Ends the test of this hook that is running, if one is.
    pub fn stop_hook_test(self, instance_id: InstanceId, kind: HookKind) {
        if let Some(stop) = self.hook_tests.lock().unwrap().get(&(instance_id, kind)) {
            stop.cancel();
        }
    }
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
            matches!(&error.failure, HookFailure::Failed { status, stderr, .. } if status.code() == Some(3) && stderr == "why\n"),
            "{error}"
        );
        assert_eq!(error.failure.summary(), "why");
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

    /// What the player is told of a failed hook: the last thing it printed,
    /// or what kept it from running.
    #[tokio::test]
    async fn a_failed_hook_is_summed_up_in_one_line() {
        let app = crate::setup_managers_for_test().await;
        let folder = tempfile::tempdir().unwrap();
        let summary = |command: &'static str, limit| {
            let (app, folder) = (app.clone(), folder.path().to_owned());
            async move {
                run(&app, HookKind::PostExit, command, &folder, limit)
                    .await
                    .expect_err("the hook fails")
                    .failure
                    .summary()
            }
        };
        assert_eq!(
            summary("sh -c 'echo first >&2; echo; echo last >&2; exit 1'", LIMIT).await,
            "last"
        );
        // Nothing on stderr: what it printed otherwise.
        assert_eq!(summary("sh -c 'echo said; exit 1'", LIMIT).await, "said");
        assert_eq!(summary("sh -c 'exit 7'", LIMIT).await, "Exit code 7.");
        assert_eq!(
            summary("sh -c 'open", LIMIT).await,
            "The command could not be read. Check its quotes."
        );
        assert_eq!(
            summary("sleep 30", Duration::from_millis(200)).await,
            "It did not finish within 0 minutes."
        );
        assert!(
            summary("/no/such/program", LIMIT)
                .await
                .contains("No such file"),
        );
    }

    async fn instance(app: &crate::managers::App, name: &str) -> (InstanceId, std::path::PathBuf) {
        use crate::domain::instance::info::{GameVersion, StandardVersion};
        let group = app.instance_manager().get_default_group().await.unwrap();
        let id = app
            .instance_manager()
            .create_instance(
                group,
                name.into(),
                false,
                crate::managers::instance::InstanceVersionSource::Version(GameVersion::Standard(
                    StandardVersion {
                        release: "1.7.10".into(),
                        modloaders: Default::default(),
                    },
                )),
                String::new(),
            )
            .await
            .unwrap();
        let (shortpath, _) = app.instance_manager().current_config(id).await.unwrap();
        let folder = app
            .settings_manager()
            .runtime_path
            .get_instances()
            .get_instance_path(&shortpath)
            .get_data_path();
        (id, folder)
    }

    /// A hook around a game is told from its start to its end, and stays
    /// until the app has shown how it ended.
    #[tokio::test]
    async fn a_hook_run_is_told_while_it_runs_and_when_it_ends() {
        let app = crate::setup_managers_for_test().await;
        let (id, folder) = instance(&app, "hook-told").await;
        tokio::fs::create_dir_all(&folder).await.unwrap();
        let manager = app.instance_manager();
        assert!(manager.hook_runs.all().is_empty());

        let running = {
            let (app, folder) = (app.clone(), folder.clone());
            tokio::spawn(async move {
                app.instance_manager()
                    .run_hook_told(
                        id,
                        "Vanilla".into(),
                        HookKind::PreLaunch,
                        "sh -c 'while [ ! -e go ]; do sleep 0.05; done'",
                        &folder,
                        LIMIT,
                    )
                    .await
            })
        };
        tokio::time::timeout(LIMIT, async {
            while manager.hook_runs.all().is_empty() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        let told = manager.hook_runs.all();
        assert_eq!(
            told,
            vec![HookRun {
                id: told[0].id,
                instance_id: id,
                name: "Vanilla".into(),
                kind: HookKind::PreLaunch,
                state: HookRunState::Running,
            }]
        );
        // Not forgotten while it still runs.
        manager.hook_runs.acknowledge(told[0].id);
        assert_eq!(manager.hook_runs.all().len(), 1);

        tokio::fs::write(folder.join("go"), b"").await.unwrap();
        running.await.unwrap().unwrap();
        assert_eq!(manager.hook_runs.all()[0].state, HookRunState::Finished);
        manager.hook_runs.acknowledge(told[0].id);
        assert!(manager.hook_runs.all().is_empty());

        manager
            .run_hook_told(
                id,
                "Vanilla".into(),
                HookKind::PostExit,
                "sh -c 'echo no remote >&2; exit 2'",
                &folder,
                LIMIT,
            )
            .await
            .expect_err("it fails");
        assert_eq!(
            manager.hook_runs.all()[0].state,
            HookRunState::Failed {
                summary: "no remote".into()
            }
        );
        assert_eq!(manager.hook_runs.all()[0].kind, HookKind::PostExit);
    }

    /// A test runs the command as the hook would run, in the instance's
    /// game folder, and returns what it did.
    #[tokio::test]
    async fn a_test_runs_the_command_in_the_game_folder_and_returns_what_it_did() {
        let app = crate::setup_managers_for_test().await;
        let (id, folder) = instance(&app, "hook-tested").await;
        let manager = app.instance_manager();

        let test = manager
            .test_hook(
                id,
                HookKind::PostExit,
                "sh -c 'touch tested; echo done; echo note >&2'".into(),
            )
            .await
            .unwrap();
        assert_eq!(test.outcome, HookTestOutcome::Finished);
        assert_eq!(test.stdout, "done\n");
        assert!(folder.join("tested").is_file());
        // A test is not a run around a game: no notice.
        assert!(manager.hook_runs.all().is_empty());

        let test = manager
            .test_hook(
                id,
                HookKind::PreLaunch,
                "sh -c 'echo out; echo why >&2; exit 127'".into(),
            )
            .await
            .unwrap();
        assert_eq!(test.outcome, HookTestOutcome::Failed { code: Some(127) });
        assert_eq!(
            (test.stdout.as_str(), test.stderr.as_str()),
            ("out\n", "why\n")
        );

        let test = manager
            .test_hook(id, HookKind::PreLaunch, "/no/such/program".into())
            .await
            .unwrap();
        assert!(
            matches!(&test.outcome, HookTestOutcome::NotRun { summary } if summary.contains("No such file")),
            "{:?}",
            test.outcome
        );
        let test = manager
            .test_hook(id, HookKind::PreLaunch, "  ".into())
            .await
            .unwrap();
        assert_eq!(
            test.outcome,
            HookTestOutcome::NotRun {
                summary: "The command is empty.".into()
            }
        );
    }

    /// Stop ends a test that is running, and its script with it. Only one
    /// test of a hook runs at a time.
    #[tokio::test]
    async fn a_running_test_is_stopped_and_only_one_runs_at_a_time() {
        let app = crate::setup_managers_for_test().await;
        let (id, folder) = instance(&app, "hook-stopped").await;
        tokio::fs::create_dir_all(&folder).await.unwrap();
        let manager = app.instance_manager();
        let test = {
            let app = app.clone();
            tokio::spawn(async move {
                app.instance_manager()
                    .test_hook(
                        id,
                        HookKind::PostExit,
                        "sh -c 'touch started; sleep 30; touch finished'".into(),
                    )
                    .await
            })
        };
        tokio::time::timeout(LIMIT, async {
            while !folder.join("started").exists() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        let second = manager
            .test_hook(id, HookKind::PostExit, "true".into())
            .await
            .expect_err("one at a time");
        assert!(second.to_string().contains("already running"), "{second}");
        // The other hook of the same instance is its own test.
        manager.stop_hook_test(id, HookKind::PreLaunch);
        assert!(!test.is_finished());

        manager.stop_hook_test(id, HookKind::PostExit);
        let stopped = tokio::time::timeout(LIMIT, test)
            .await
            .expect("stop ends the test")
            .unwrap()
            .unwrap();
        assert_eq!(stopped.outcome, HookTestOutcome::Stopped);
        assert!(stopped.millis < 20_000);
        assert!(!folder.join("finished").exists());
        // And it can be tested again.
        let again = manager
            .test_hook(id, HookKind::PostExit, "true".into())
            .await
            .unwrap();
        assert_eq!(again.outcome, HookTestOutcome::Finished);
    }

    #[test]
    fn only_the_end_of_a_long_output_is_shown_from_a_whole_line_on() {
        assert_eq!(tail("short\n".into()), "short\n");
        let long = format!("{}\nlast line\n", "x".repeat(SHOWN_OUTPUT * 2));
        assert_eq!(tail(long), "last line\n");
        let lines = "0123456789\n".repeat(SHOWN_OUTPUT / 4);
        let shown = tail(lines);
        assert!(shown.len() <= SHOWN_OUTPUT);
        assert!(shown.starts_with("0123456789\n"));
    }
}
