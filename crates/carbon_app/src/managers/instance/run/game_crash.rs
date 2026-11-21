//! Telling the player that the game crashed or did not start.
//!
//! The launcher sees how every game it started ends. An exit with an error
//! is not enough to call a crash, though: a game ended from the Task Manager
//! exits with an error too, and so does one the system ends for lack of
//! memory, and neither can be told from the other. A notice is raised only
//! when the exit also carries a sign that the game itself failed (see
//! [`classify`]), so that it is never raised for something the player did.

use crate::domain::instance::InstanceId;
use std::collections::VecDeque;
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, SystemTime};

/// What the game prints when it crashes: the client's own line, the head of
/// a crash report written to the log, and the Java runtime's fatal error.
const CRASH_LINES: [&str; 3] = [
    "Game crashed! Crash report",
    "---- Minecraft Crash Report ----",
    "A fatal error has been detected by the Java Runtime Environment",
];

/// A game that ends this soon after it started never got going.
const START_UP: Duration = Duration::from_secs(60);

/// How many notices wait for the app at most; the oldest go first.
const KEPT: usize = 20;

pub(in crate::managers::instance) fn says_crashed(line: &str) -> bool {
    CRASH_LINES.iter().any(|sign| line.contains(sign))
}

/// How a game the launcher started ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameExit {
    /// It ended without an error.
    pub success: bool,
    /// Its exit code, when it has one.
    pub code: Option<i32>,
    /// The system ended it for a fault of its own: a crash signal, or a
    /// Windows exception status as its exit code.
    pub faulted: bool,
    /// Stop was pressed.
    pub stopped: bool,
    /// It printed that it crashed (see [`says_crashed`]).
    pub said_crashed: bool,
    /// It wrote a crash report, or the Java runtime an error file, since it
    /// started.
    pub wrote_report: bool,
    pub ran_for: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrashKind {
    /// The game failed while running. `report` is whether it left a crash
    /// report.
    Crashed { report: bool },
    /// The game ended with an error within a minute of starting, with no
    /// word of why.
    DidNotStart { seconds: u32, code: Option<i32> },
}

/// Whether `exit` is told to the player, and as what. Never for an exit
/// without an error or after Stop. Otherwise only with a sign that the game
/// itself failed: it wrote a report, said so, or was ended for a fault of
/// its own; or, with none of these, it ended within a minute of starting.
pub fn classify(exit: &GameExit) -> Option<CrashKind> {
    if exit.success || exit.stopped {
        return None;
    }
    if exit.wrote_report || exit.said_crashed || exit.faulted {
        return Some(CrashKind::Crashed {
            report: exit.wrote_report,
        });
    }
    (exit.ran_for < START_UP).then(|| CrashKind::DidNotStart {
        seconds: exit.ran_for.as_secs() as u32,
        code: exit.code,
    })
}

/// Whether the system ended the process for a fault of its own.
pub fn faulted(status: &std::process::ExitStatus) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        matches!(
            status.signal(),
            Some(libc::SIGSEGV | libc::SIGABRT | libc::SIGBUS | libc::SIGILL | libc::SIGFPE)
        )
    }
    #[cfg(windows)]
    {
        // An NTSTATUS error, such as 0xC0000005 for an access violation.
        // Ending a process from the Task Manager exits with 1.
        status.code().is_some_and(|code| code as u32 >= 0xC000_0000)
    }
}

fn written_since(path: &Path, since: SystemTime) -> bool {
    std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .is_ok_and(|modified| modified >= since)
}

/// Whether the game left a crash report in `crash_reports`, or the Java
/// runtime an error file (`hs_err_pid*.log`) in `data`, since `since`.
pub async fn wrote_report(data: &Path, crash_reports: &Path, since: SystemTime) -> bool {
    let (data, crash_reports) = (data.to_owned(), crash_reports.to_owned());
    tokio::task::spawn_blocking(move || {
        let files = |folder: &Path| {
            std::fs::read_dir(folder)
                .into_iter()
                .flatten()
                .flatten()
                .map(|entry| entry.path())
                .collect::<Vec<_>>()
        };
        files(&crash_reports)
            .iter()
            .any(|path| path.is_file() && written_since(path, since))
            || files(&data).iter().any(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("hs_err_pid") && name.ends_with(".log"))
                    && written_since(path, since)
            })
    })
    .await
    .unwrap_or(false)
}

/// One crash the app has not shown yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameCrash {
    pub id: u32,
    pub instance_id: InstanceId,
    /// The instance's name when it crashed.
    pub name: String,
    pub kind: CrashKind,
}

/// The crashes waiting for the app to show them.
#[derive(Debug, Default)]
pub struct GameCrashes {
    next: AtomicU32,
    pending: Mutex<VecDeque<GameCrash>>,
}

impl GameCrashes {
    pub fn push(&self, instance_id: InstanceId, name: String, kind: CrashKind) {
        let mut pending = self.pending.lock().unwrap();
        if pending.len() == KEPT {
            pending.pop_front();
        }
        pending.push_back(GameCrash {
            id: self.next.fetch_add(1, Ordering::Relaxed),
            instance_id,
            name,
            kind,
        });
    }

    pub fn pending(&self) -> Vec<GameCrash> {
        self.pending.lock().unwrap().iter().cloned().collect()
    }

    pub fn acknowledge(&self, id: u32) {
        self.pending.lock().unwrap().retain(|crash| crash.id != id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exit() -> GameExit {
        GameExit {
            success: false,
            code: Some(1),
            faulted: false,
            stopped: false,
            said_crashed: false,
            wrote_report: false,
            ran_for: Duration::from_secs(3 * 60 * 60),
        }
    }

    /// A crash after hours of play is told whenever it leaves
    /// a sign, and what the player did themselves never is.
    #[test]
    fn a_crash_is_told_only_with_a_sign_that_the_game_itself_failed() {
        // An error exit alone, late in a session: ended from the Task
        // Manager, or by the system for lack of memory. Not told.
        assert_eq!(classify(&exit()), None);
        assert_eq!(
            classify(&GameExit {
                wrote_report: true,
                ..exit()
            }),
            Some(CrashKind::Crashed { report: true })
        );
        assert_eq!(
            classify(&GameExit {
                said_crashed: true,
                ..exit()
            }),
            Some(CrashKind::Crashed { report: false })
        );
        assert_eq!(
            classify(&GameExit {
                faulted: true,
                code: None,
                ..exit()
            }),
            Some(CrashKind::Crashed { report: false })
        );
        // Never after Stop, whatever it left behind, and never for a game
        // that ended without an error.
        assert_eq!(
            classify(&GameExit {
                stopped: true,
                wrote_report: true,
                said_crashed: true,
                faulted: true,
                ran_for: Duration::from_secs(2),
                ..exit()
            }),
            None
        );
        assert_eq!(
            classify(&GameExit {
                success: true,
                code: Some(0),
                wrote_report: true,
                ran_for: Duration::from_secs(2),
                ..exit()
            }),
            None
        );
    }

    /// A game that ends with an error within a minute, with no word of why,
    /// did not start. After a minute the same exit is not told.
    #[test]
    fn an_error_exit_within_a_minute_is_a_game_that_did_not_start() {
        let early = |seconds| GameExit {
            ran_for: Duration::from_secs(seconds),
            ..exit()
        };
        assert_eq!(
            classify(&early(3)),
            Some(CrashKind::DidNotStart {
                seconds: 3,
                code: Some(1)
            })
        );
        assert_eq!(
            classify(&GameExit {
                code: None,
                ..early(59)
            }),
            Some(CrashKind::DidNotStart {
                seconds: 59,
                code: None
            })
        );
        assert_eq!(classify(&early(60)), None);
        // With a report it crashed, however early.
        assert_eq!(
            classify(&GameExit {
                wrote_report: true,
                ..early(3)
            }),
            Some(CrashKind::Crashed { report: true })
        );
    }

    #[test]
    fn the_games_own_crash_lines_are_recognised() {
        assert!(says_crashed(
            "#@!@# Game crashed! Crash report saved to: #@!@# /x/crash-reports/crash.txt"
        ));
        assert!(says_crashed(
            "#@?@# Game crashed! Crash report could not be saved. #@?@#"
        ));
        assert!(says_crashed("---- Minecraft Crash Report ----\n// Oops."));
        assert!(says_crashed(
            "# A fatal error has been detected by the Java Runtime Environment:"
        ));
        assert!(!says_crashed("Stopping!"));
        assert!(!says_crashed("Saving chunks for level 'New World'"));
    }

    #[cfg(unix)]
    #[test]
    fn only_a_fault_of_the_process_itself_counts_as_one() {
        use std::os::unix::process::ExitStatusExt;
        let signalled = |signal| std::process::ExitStatus::from_raw(signal);
        assert!(faulted(&signalled(libc::SIGSEGV)));
        assert!(faulted(&signalled(libc::SIGABRT)));
        // What a person or the system's memory killer sends.
        assert!(!faulted(&signalled(libc::SIGKILL)));
        assert!(!faulted(&signalled(libc::SIGTERM)));
        // An ordinary exit with an error code.
        assert!(!faulted(&std::process::ExitStatus::from_raw(1 << 8)));
    }

    /// Only files written since the game started count, in the two places
    /// the game and the Java runtime write them.
    #[tokio::test]
    async fn a_report_counts_only_when_written_since_the_game_started() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("instance");
        let reports = data.join("crash-reports");
        std::fs::create_dir_all(&reports).unwrap();
        std::fs::write(reports.join("crash-old.txt"), b"old").unwrap();
        std::fs::write(data.join("hs_err_pid1.log"), b"old").unwrap();
        std::fs::write(data.join("options.txt"), b"x").unwrap();
        let old = std::fs::metadata(reports.join("crash-old.txt"))
            .unwrap()
            .modified()
            .unwrap();
        let started = old + Duration::from_secs(10);
        assert!(!wrote_report(&data, &reports, started).await);
        // A folder that is not there yet is no report.
        assert!(!wrote_report(&data, &data.join("missing"), started).await);

        let stamp = |path: &Path| {
            std::fs::File::options()
                .write(true)
                .open(path)
                .unwrap()
                .set_modified(started + Duration::from_secs(5))
                .unwrap()
        };
        std::fs::write(reports.join("crash-new.txt"), b"new").unwrap();
        stamp(&reports.join("crash-new.txt"));
        assert!(wrote_report(&data, &reports, started).await);
        std::fs::remove_file(reports.join("crash-new.txt")).unwrap();
        assert!(!wrote_report(&data, &reports, started).await);

        std::fs::write(data.join("hs_err_pid4242.log"), b"new").unwrap();
        stamp(&data.join("hs_err_pid4242.log"));
        assert!(wrote_report(&data, &reports, started).await);
        // Another file changed since is not a report.
        std::fs::remove_file(data.join("hs_err_pid4242.log")).unwrap();
        stamp(&data.join("options.txt"));
        assert!(!wrote_report(&data, &reports, started).await);
    }

    #[test]
    fn crashes_wait_until_acknowledged_and_the_oldest_give_way() {
        let crashes = GameCrashes::default();
        for n in 0..(KEPT as i32 + 2) {
            crashes.push(
                InstanceId(n),
                format!("instance {n}"),
                CrashKind::Crashed { report: true },
            );
        }
        let pending = crashes.pending();
        assert_eq!(pending.len(), KEPT);
        assert_eq!(pending[0].instance_id, InstanceId(2));
        assert_eq!(pending[0].name, "instance 2");
        crashes.acknowledge(pending[0].id);
        assert_eq!(crashes.pending().len(), KEPT - 1);
        assert_eq!(crashes.pending()[0].instance_id, InstanceId(3));
    }
}
