use super::*;
#[derive(Default)]
pub(super) struct RefreshSchedule {
    last_admitted: Option<String>,
    retries: HashMap<String, (Instant, Duration)>,
}
impl RefreshSchedule {
    pub fn order(
        &self,
        accounts: &mut [account_repo::AccountRow],
        gdl: Option<&str>,
        active: Option<&str>,
        _now: Instant,
    ) {
        accounts.sort_by_key(|a| {
            if Some(a.uuid.as_str()) == gdl {
                0
            } else if Some(a.uuid.as_str()) == active {
                1
            } else {
                2
            }
        });
        if let Some(index) = self
            .last_admitted
            .as_ref()
            .and_then(|id| accounts.iter().position(|a| &a.uuid == id))
        {
            let count = accounts.len();
            accounts.rotate_left((index + 1) % count);
        }
    }
    pub fn eligible(&self, uuid: &str, now: Instant) -> bool {
        self.retries
            .get(uuid)
            .is_none_or(|(deadline, _)| now >= *deadline)
    }
    pub fn admitted(&mut self, uuid: &str) {
        self.last_admitted = Some(uuid.to_owned());
    }
    pub fn failed(&mut self, uuid: &str, now: Instant) {
        let delay = self
            .retries
            .get(uuid)
            .map(|(_, delay)| (*delay * 2).min(Duration::from_secs(300)))
            .unwrap_or(Duration::from_secs(60));
        self.retries.insert(uuid.to_owned(), (now + delay, delay));
    }
    pub fn success(&mut self, uuid: &str) {
        self.retries.remove(uuid);
    }
    pub fn remove(&mut self, uuid: &str) {
        self.retries.remove(uuid);
        if self.last_admitted.as_deref() == Some(uuid) {
            self.last_admitted = None;
        }
    }
}
#[cfg(test)]
mod task19_schedule_tests {
    use super::super::publication::task19_publication_tests::{account, database};
    use super::*;
    #[tokio::test]
    async fn fast_failed_priority_account_does_not_starve_other_accounts() {
        let (_dir, db) = database().await;
        let publication = super::super::publication::Publication::default();
        for uuid in ["a", "b", "c"] {
            publication
                .save(&db, account(uuid, "dummy"), |_| {})
                .await
                .unwrap();
        }
        let mut schedule = RefreshSchedule::default();
        let mut rows = account_repo::get_accounts_by_last_used(&db).await.unwrap();
        schedule.order(&mut rows, Some("a"), Some("b"), Instant::now());
        assert_eq!(rows[0].uuid, "a");
        schedule.admitted("a"); // A has finished a fast transient failure before the next sweep.
        schedule.order(
            &mut rows,
            Some("a"),
            Some("b"),
            Instant::now() + Duration::from_secs(30),
        );
        assert_eq!(rows[0].uuid, "b", "fast failure monopolizes the next sweep");
    }
    #[tokio::test]
    async fn transient_backoff_is_bounded_and_rotation_reaches_b_and_c() {
        let (_dir, db) = database().await;
        let publication = super::super::publication::Publication::default();
        for uuid in ["a", "b", "c"] {
            publication
                .save(&db, account(uuid, "dummy"), |_| {})
                .await
                .unwrap();
        }
        let now = Instant::now();
        let mut schedule = RefreshSchedule::default();
        let mut rows = account_repo::get_accounts_by_last_used(&db).await.unwrap();
        schedule.order(&mut rows, Some("a"), Some("b"), now);
        assert_eq!(rows[0].uuid, "a");
        schedule.admitted("a");
        schedule.failed("a", now);
        assert!(!schedule.eligible("a", now + Duration::from_secs(30)));
        schedule.order(&mut rows, Some("a"), Some("b"), now);
        assert_eq!(rows[0].uuid, "b");
        schedule.admitted("b");
        schedule.order(&mut rows, Some("a"), Some("b"), now);
        assert_eq!(rows[0].uuid, "c");
        schedule.admitted("c");
        schedule.order(
            &mut rows,
            Some("a"),
            Some("b"),
            now + Duration::from_secs(60),
        );
        assert_eq!(rows[0].uuid, "a");
        assert!(schedule.eligible("a", now + Duration::from_secs(60)));
        for _ in 0..12 {
            schedule.failed("a", now);
        }
        assert!(!schedule.eligible("a", now + Duration::from_secs(299)));
        assert!(schedule.eligible("a", now + Duration::from_secs(300)));
        schedule.success("a");
        assert!(schedule.eligible("a", now));
        schedule.failed("a", now);
        schedule.remove("a");
        assert!(schedule.eligible("a", now));
    }
}
