//! Serialized local account publication. No provider operations belong under this gate.
use super::*;
use carbon_repos::db_exec::Db;
#[derive(Default)]
pub(super) struct Publication {
    pub gate: Mutex<State>,
}
#[derive(Default)]
pub(super) struct State {
    generations: HashMap<String, uuid::Uuid>,
    pub refreshing: HashMap<String, EnrollmentTask>,
    pub schedule: super::schedule::RefreshSchedule,
}
#[derive(Clone, Debug)]
pub(super) struct Ticket {
    pub uuid: String,
    generation: uuid::Uuid,
}
impl State {
    pub fn capture(&mut self, uuid: &str) -> Ticket {
        Ticket {
            uuid: uuid.to_owned(),
            generation: *self
                .generations
                .entry(uuid.to_owned())
                .or_insert_with(uuid::Uuid::new_v4),
        }
    }
    pub fn owns(&self, ticket: &Ticket) -> bool {
        self.generations.get(&ticket.uuid) == Some(&ticket.generation)
    }
    pub fn retire(&mut self, uuid: &str) {
        self.generations.remove(uuid);
        self.refreshing.remove(uuid); // Drop aborts obsolete transport; publication still checks ownership.
        self.schedule.remove(uuid);
    }
    pub async fn delete(&mut self, db: &Db, uuid: &str) -> anyhow::Result<usize> {
        let rows = account_repo::delete_account(db, uuid).await?;
        self.retire(uuid);
        Ok(rows)
    }
    pub async fn save(
        &mut self,
        db: &Db,
        account: FullAccount,
        invalidate: impl Fn(Option<serde_json::Value>),
    ) -> anyhow::Result<Ticket> {
        let uuid = account.uuid.clone();
        persist(db, account, invalidate).await?;
        self.retire(&uuid);
        Ok(self.capture(&uuid))
    }
    async fn refresh(
        &mut self,
        db: &Db,
        ticket: &Ticket,
        account: FullAccount,
    ) -> anyhow::Result<bool> {
        if !self.owns(ticket) {
            return Ok(false);
        }
        ensure!(
            account.uuid == ticket.uuid,
            "refresh returned a different account identity"
        );
        let Some(current) = account_repo::get_account(db, &ticket.uuid).await? else {
            return Ok(false);
        };
        let FullAccountType::Microsoft {
            access_token,
            token_expires,
            refresh_token,
            id_token,
            gdl_token,
            ..
        } = account.type_
        else {
            bail!("refresh did not return Microsoft credentials");
        };
        let rows = account_repo::refresh_account_credentials(
            db,
            &ticket.uuid,
            current.ms_refresh_token.as_deref(),
            &access_token,
            DbDateTime(token_expires.into()),
            refresh_token.as_deref(),
            id_token.as_deref(),
            gdl_token.as_deref(),
        )
        .await?;
        Ok(rows == 1)
    }
    async fn reject(&mut self, db: &Db, ticket: &Ticket) -> anyhow::Result<bool> {
        if !self.owns(ticket) {
            return Ok(false);
        }
        let Some(current) = account_repo::get_account(db, &ticket.uuid).await? else {
            return Ok(false);
        };
        Ok(account_repo::reject_account_credentials(
            db,
            &ticket.uuid,
            current.ms_refresh_token.as_deref(),
        )
        .await?
            == 1)
    }
}
impl Publication {
    pub async fn admit_refresh<H>(
        &self,
        db: &Db,
        uuid: &str,
        automatic: bool,
        now: Instant,
        start: impl FnOnce(String, Ticket, String) -> (EnrollmentTask, H),
        invalidate: impl Fn(Option<serde_json::Value>),
    ) -> anyhow::Result<H> {
        let mut state = self.gate.lock().await;
        if state.refreshing.contains_key(uuid) {
            bail!(RefreshAccountError::AlreadyRefreshing);
        }
        if automatic && !state.schedule.eligible(uuid, now) {
            bail!(RefreshAccountError::Backoff);
        }
        let account = account_repo::get_account(db, uuid)
            .await?
            .ok_or(RefreshAccountError::NoAccount)?;
        let token = account
            .ms_refresh_token
            .ok_or(RefreshAccountError::NoRefreshToken)?;
        let ticket = state.capture(uuid);
        let operation = uuid::Uuid::new_v4().to_string();
        let (mut task, handle) = start(token, ticket, operation.clone());
        task.id = operation;
        state.refreshing.insert(uuid.to_owned(), task);
        if automatic {
            state.schedule.admitted(uuid);
        }
        invalidate(None);
        Ok(handle)
    }
    pub async fn publish_gdl_token(
        &self,
        db: &Db,
        ticket: &Ticket,
        expected_id: &str,
        token: &str,
        invalidate: impl Fn(Option<serde_json::Value>),
    ) -> anyhow::Result<bool> {
        let state = self.gate.lock().await;
        if !state.owns(ticket) {
            return Ok(false);
        }
        let changed =
            account_repo::set_account_gdl_token_owned(db, &ticket.uuid, expected_id, token).await?
                == 1;
        if changed {
            invalidate(None);
        }
        Ok(changed)
    }

    pub async fn publish_enrollment<F, Fut>(
        &self,
        db: &Db,
        account: FullAccount,
        select: F,
        invalidate: impl Fn(Option<serde_json::Value>),
    ) -> anyhow::Result<Ticket>
    where
        F: FnOnce(String) -> Fut,
        Fut: std::future::Future<Output = anyhow::Result<()>>,
    {
        let mut state = self.gate.lock().await;
        let uuid = account.uuid.clone();
        let ticket = state.save(db, account, invalidate).await?;
        select(uuid).await?;
        Ok(ticket)
    }

    #[cfg(test)]
    pub async fn save(
        &self,
        db: &Db,
        account: FullAccount,
        invalidate: impl Fn(Option<serde_json::Value>),
    ) -> anyhow::Result<()> {
        self.gate.lock().await.save(db, account, invalidate).await?;
        Ok(())
    }
    #[cfg(test)]
    pub async fn delete(&self, db: &Db, uuid: &str) -> anyhow::Result<usize> {
        self.gate.lock().await.delete(db, uuid).await
    }
    #[cfg(test)]
    pub async fn refresh(
        &self,
        db: &Db,
        ticket: &Ticket,
        account: FullAccount,
        invalidate: impl Fn(Option<serde_json::Value>),
    ) -> anyhow::Result<()> {
        if self.gate.lock().await.refresh(db, ticket, account).await? {
            invalidate(None);
        }
        Ok(())
    }
    /// Progress callbacks and terminal callbacks use the exact same admitted task.
    /// No refresh-map/status guard is retained across database publication.
    pub async fn complete_refresh(
        &self,
        db: &Db,
        ticket: &Ticket,
        operation: &str,
        invalidate: impl Fn(Option<serde_json::Value>),
    ) -> anyhow::Result<bool> {
        let mut state = self.gate.lock().await;
        if !state.owns(ticket) {
            return Ok(false);
        }
        enum Outcome {
            Success(api::FullAccount),
            Rejected,
            Transient,
        }
        let outcome = {
            let Some(task) = state
                .refreshing
                .get(&ticket.uuid)
                .filter(|task| task.id == operation)
            else {
                return Ok(false);
            };
            let status = task.status.read().await;
            match &*status {
                EnrollmentStatus::Complete(account) => Outcome::Success(account.clone()),
                EnrollmentStatus::Failed(Ok(_)) => Outcome::Rejected,
                EnrollmentStatus::Failed(Err(e)) if e.is::<api::RefreshRejected>() => {
                    Outcome::Rejected
                }
                EnrollmentStatus::Failed(Err(_)) => Outcome::Transient,
                _ => return Ok(false),
            }
        };
        let result = match outcome {
            Outcome::Success(account) => state.refresh(db, ticket, account.into()).await,
            Outcome::Rejected => state.reject(db, ticket).await,
            Outcome::Transient => {
                state.schedule.failed(&ticket.uuid, Instant::now());
                Ok(false)
            }
        };
        match &result {
            Ok(true) => state.schedule.success(&ticket.uuid),
            Err(_) => state.schedule.failed(&ticket.uuid, Instant::now()),
            _ => {}
        }
        // Removal only follows the matched task, while retaining publication ownership.
        state.refreshing.remove(&ticket.uuid);
        invalidate(None);
        result
    }
}
async fn persist(
    db: &Db,
    account: FullAccount,
    invalidate: impl Fn(Option<serde_json::Value>),
) -> anyhow::Result<()> {
    let uuid = account.uuid.clone();
    let db_account = account_repo::get_account(db, &uuid).await?;

    if db_account.is_some() {
        // don't change lastUsed
        info!("Updating account information for {:?}", &account.uuid);

        match account.type_ {
            FullAccountType::Offline => {
                account_repo::update_account_offline(db, &account.uuid, &account.username).await?;
            }
            FullAccountType::Microsoft {
                access_token,
                refresh_token,
                token_expires,
                id_token,
                gdl_token,
                email: _,
                skin_id,
            } => {
                let token_expires = DbDateTime(token_expires.with_timezone(&FixedOffset::east(0)));
                account_repo::update_account_microsoft(
                    db,
                    &account.uuid,
                    &account.username,
                    &access_token,
                    Some(token_expires),
                    refresh_token.as_deref(),
                    id_token.as_deref(),
                    gdl_token.as_deref(),
                    skin_id.as_deref(),
                )
                .await?;
            }
        }

        invalidate(None);
    } else {
        info!("Creating account {:?}", &account.uuid);

        let last_used = DbDateTime(Utc::now().into());
        match account.type_ {
            FullAccountType::Offline => {
                account_repo::insert_account_offline(
                    db,
                    &account.uuid,
                    &account.username,
                    last_used,
                    None,
                )
                .await?;
            }
            FullAccountType::Microsoft {
                access_token,
                refresh_token,
                token_expires,
                id_token,
                gdl_token,
                email: _,
                skin_id,
            } => {
                let token_expires = DbDateTime(token_expires.with_timezone(&FixedOffset::east(0)));
                account_repo::insert_account_microsoft(
                    db,
                    &account.uuid,
                    &account.username,
                    last_used,
                    &access_token,
                    Some(token_expires),
                    refresh_token.as_deref(),
                    id_token.as_deref(),
                    gdl_token.as_deref(),
                    skin_id.as_deref(),
                )
                .await?;
            }
        }

        invalidate(None);
    }

    Ok(())
}

#[cfg(test)]
pub(super) mod task19_publication_tests {
    use super::*;
    pub async fn database() -> (tempfile::TempDir, Db) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("accounts.db");
        {
            let mut conn = rusqlite::Connection::open(&path).unwrap();
            carbon_repos::get_migrations()
                .0
                .to_latest(&mut conn)
                .unwrap();
        }
        let db = Db::open(&path, 2, true).unwrap();
        (dir, db)
    }
    pub fn account(uuid: &str, token: &str) -> FullAccount {
        FullAccount {
            uuid: uuid.into(),
            username: "Example".into(),
            last_used: Utc::now().into(),
            type_: FullAccountType::Microsoft {
                access_token: token.into(),
                refresh_token: Some(format!("dummy-refresh-{token}")),
                id_token: Some("dummy-id".into()),
                gdl_token: None,
                email: None,
                token_expires: Utc::now() + chrono::Duration::hours(24),
                skin_id: None,
            },
        }
    }
    #[tokio::test]
    async fn removed_refresh_never_recreates_account() {
        let (_dir, db) = database().await;
        let publication = Publication::default();
        publication
            .save(&db, account("a", "old"), |_| {})
            .await
            .unwrap();
        publication
            .save(&db, account("b", "healthy"), |_| {})
            .await
            .unwrap();
        let ticket = publication.gate.lock().await.capture("a");
        publication.delete(&db, "a").await.unwrap();
        publication
            .refresh(&db, &ticket, account("a", "late"), |_| {})
            .await
            .unwrap();
        assert!(
            account_repo::get_account(&db, "a").await.unwrap().is_none(),
            "old refresh reinserted removed credentials"
        );
        assert_eq!(
            account_repo::get_account(&db, "b")
                .await
                .unwrap()
                .unwrap()
                .access_token
                .as_deref(),
            Some("healthy")
        );
    }
    #[tokio::test]
    async fn replaced_generation_rejects_old_refresh() {
        let (_dir, db) = database().await;
        let publication = Publication::default();
        publication
            .save(&db, account("a", "old"), |_| {})
            .await
            .unwrap();
        let ticket = publication.gate.lock().await.capture("a");
        publication.delete(&db, "a").await.unwrap();
        publication
            .save(&db, account("a", "new-login"), |_| {})
            .await
            .unwrap();
        publication
            .refresh(&db, &ticket, account("a", "late"), |_| {})
            .await
            .unwrap();
        assert_eq!(
            account_repo::get_account(&db, "a")
                .await
                .unwrap()
                .unwrap()
                .access_token
                .as_deref(),
            Some("new-login")
        );
    }
    #[tokio::test]
    async fn refreshed_list_invalidation_uses_no_input() {
        let (_dir, db) = database().await;
        let publication = Publication::default();
        publication
            .save(&db, account("a", "old"), |_| {})
            .await
            .unwrap();
        let ticket = publication.gate.lock().await.capture("a");
        let events = std::sync::Mutex::new(Vec::new());
        publication
            .refresh(&db, &ticket, account("a", "new"), |args| {
                events.lock().unwrap().push(args)
            })
            .await
            .unwrap();
        assert_eq!(
            *events.lock().unwrap(),
            vec![None],
            "refresh event misses no-input account query"
        );
    }
    pub fn api_account(token: &str) -> api::FullAccount {
        api::FullAccount {
            ms: api::MsAuth {
                access_token: "dummy-ms".into(),
                id_token: "dummy-id".into(),
                refresh_token: format!("dummy-refresh-{token}"),
                expires_at: Utc::now() + chrono::Duration::hours(24),
            },
            mc: api::McAccount {
                auth: api::McAuth {
                    access_token: token.into(),
                    expires_at: Utc::now() + chrono::Duration::hours(24),
                },
                entitlement: api::McEntitlement::Owned,
                profile: api::McProfile {
                    uuid: "a".into(),
                    username: "ProviderName".into(),
                    skin: None,
                },
            },
            gdl_token: None,
        }
    }
    async fn admit(publication: &Publication, status: EnrollmentStatus) -> (Ticket, String) {
        let mut state = publication.gate.lock().await;
        let ticket = state.capture("a");
        let task = EnrollmentTask::inert(status);
        let operation = task.id.clone();
        state.refreshing.insert("a".into(), task);
        (ticket, operation)
    }
    #[tokio::test]
    async fn completion_preserves_last_used_and_current_profile_then_delete_wins() {
        let (_dir, db) = database().await;
        let publication = Publication::default();
        publication
            .save(&db, account("a", "old"), |_| {})
            .await
            .unwrap();
        let before = account_repo::get_account(&db, "a").await.unwrap().unwrap();
        account_repo::update_account_profile(&db, "a", "CurrentName", Some("current-skin"))
            .await
            .unwrap();
        let (ticket, op) =
            admit(&publication, EnrollmentStatus::Complete(api_account("new"))).await;
        assert!(
            publication
                .complete_refresh(&db, &ticket, &op, |_| {})
                .await
                .unwrap()
        );
        let after = account_repo::get_account(&db, "a").await.unwrap().unwrap();
        assert_eq!(after.last_used, before.last_used);
        assert_eq!(after.username, "CurrentName");
        assert_eq!(after.skin_id.as_deref(), Some("current-skin"));
        assert_eq!(after.access_token.as_deref(), Some("new"));
        assert!(publication.gate.lock().await.refreshing.is_empty());
        publication.delete(&db, "a").await.unwrap();
        assert!(account_repo::get_account(&db, "a").await.unwrap().is_none());
    }
    #[tokio::test]
    async fn retired_callbacks_cannot_clear_new_credentials_or_offline_replacement() {
        for offline in [false, true] {
            let (_dir, db) = database().await;
            let publication = Publication::default();
            publication
                .save(&db, account("a", "old"), |_| {})
                .await
                .unwrap();
            let (ticket, op) = admit(
                &publication,
                EnrollmentStatus::Failed(Err(api::RefreshRejected.into())),
            )
            .await;
            publication.delete(&db, "a").await.unwrap();
            let mut replacement = account("a", "replacement");
            if offline {
                replacement.type_ = FullAccountType::Offline;
            }
            publication.save(&db, replacement, |_| {}).await.unwrap();
            let before = account_repo::get_account(&db, "a").await.unwrap().unwrap();
            assert!(
                !publication
                    .complete_refresh(&db, &ticket, &op, |_| panic!("obsolete event"))
                    .await
                    .unwrap()
            );
            assert_eq!(
                account_repo::get_account(&db, "a").await.unwrap().unwrap(),
                before
            );
            assert!(publication.gate.lock().await.refreshing.is_empty());
        }
    }
    #[tokio::test]
    async fn final_publication_gate_serializes_deletion_and_callbacks() {
        let (_dir, db) = database().await;
        let db = Arc::new(db);
        let publication = Arc::new(Publication::default());
        publication
            .save(&db, account("a", "old"), |_| {})
            .await
            .unwrap();
        let (ticket, op) = admit(
            &publication,
            EnrollmentStatus::Complete(api_account("late")),
        )
        .await;
        let mut gate = publication.gate.lock().await;
        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let callback = {
            let publication = publication.clone();
            let db = db.clone();
            tokio::spawn(async move {
                entered_tx.send(()).unwrap();
                publication
                    .complete_refresh(&db, &ticket, &op, |_| panic!("deleted completion event"))
                    .await
            })
        };
        entered_rx.await.unwrap();
        gate.delete(&db, "a").await.unwrap();
        assert!(gate.refreshing.is_empty());
        drop(gate);
        assert!(!callback.await.unwrap().unwrap());
        assert!(account_repo::get_account(&db, "a").await.unwrap().is_none());
    }
    #[tokio::test]
    async fn rejection_invalidates_but_transient_failure_preserves_credentials_and_backs_off() {
        for permanent in [false, true] {
            let (_dir, db) = database().await;
            let publication = Publication::default();
            publication
                .save(&db, account("a", "healthy"), |_| {})
                .await
                .unwrap();
            let before = account_repo::get_account(&db, "a").await.unwrap().unwrap();
            let error = if permanent {
                api::RefreshRejected.into()
            } else {
                anyhow!("synthetic connectivity failure")
            };
            let (ticket, op) = admit(&publication, EnrollmentStatus::Failed(Err(error))).await;
            publication
                .complete_refresh(&db, &ticket, &op, |args| assert!(args.is_none()))
                .await
                .unwrap();
            let after = account_repo::get_account(&db, "a").await.unwrap().unwrap();
            assert_eq!(after.access_token, before.access_token);
            if permanent {
                assert!(after.ms_refresh_token.is_none());
                assert!(after.id_token.is_none());
            } else {
                assert_eq!(after, before);
                assert!(
                    !publication
                        .gate
                        .lock()
                        .await
                        .schedule
                        .eligible("a", Instant::now())
                );
            }
        }
    }
    #[tokio::test]
    async fn captured_account_events_match_the_live_no_input_wire_contract() {
        let (_dir, db) = database().await;
        let publication = Publication::default();
        let events = std::sync::Mutex::new(Vec::new());
        let capture = |args| {
            events
                .lock()
                .unwrap()
                .push(crate::api::InvalidationEvent::new(GET_ACCOUNTS.full, args))
        };
        publication
            .save(&db, account("a", "old"), &capture)
            .await
            .unwrap();
        let (ticket, op) = publication
            .admit_refresh(
                &db,
                "a",
                false,
                Instant::now(),
                |_, ticket, op| {
                    (
                        EnrollmentTask::inert(EnrollmentStatus::Failed(Err(anyhow!(
                            "synthetic failure"
                        )))),
                        (ticket, op),
                    )
                },
                &capture,
            )
            .await
            .unwrap();
        publication
            .complete_refresh(&db, &ticket, &op, &capture)
            .await
            .unwrap();
        let (ticket, op) = publication
            .admit_refresh(
                &db,
                "a",
                false,
                Instant::now(),
                |_, ticket, op| {
                    (
                        EnrollmentTask::inert(EnrollmentStatus::Complete(api_account("new"))),
                        (ticket, op),
                    )
                },
                &capture,
            )
            .await
            .unwrap();
        publication
            .complete_refresh(&db, &ticket, &op, &capture)
            .await
            .unwrap();
        let bytes = serde_json::to_vec(&*events.lock().unwrap()).unwrap();
        // queryInvalidation.integration.ts consumes this file; `pnpm
        // test:account-contract` points both sides at one fresh path.
        let fixture = std::env::var_os("TASK19_ACCOUNT_EVENTS")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("task19-account-events.json"));
        if let Some(parent) = fixture.parent() {
            std::fs::create_dir_all(parent).unwrap_or_else(|error| {
                panic!(
                    "cannot create {} for TASK19_ACCOUNT_EVENTS: {error}",
                    parent.display()
                )
            });
        }
        std::fs::write(&fixture, bytes).unwrap_or_else(|error| {
            panic!(
                "cannot write TASK19_ACCOUNT_EVENTS to {}: {error}",
                fixture.display()
            )
        });
        assert_eq!(events.lock().unwrap().len(), 5);
        assert!(
            events
                .lock()
                .unwrap()
                .iter()
                .all(|event| event.args.is_none())
        );
    }

    #[tokio::test]
    async fn admission_excludes_inflight_honors_backoff_and_allows_explicit_recovery() {
        let (_dir, db) = database().await;
        let publication = Publication::default();
        for id in ["a", "b"] {
            publication
                .save(&db, account(id, "dummy"), |_| {})
                .await
                .unwrap();
        }
        let start = |_: String, ticket: Ticket, operation: String| {
            (
                EnrollmentTask::inert(EnrollmentStatus::McProfile),
                (ticket, operation),
            )
        };
        let (ticket, op) = publication
            .admit_refresh(&db, "a", true, Instant::now(), start, |args| {
                assert!(args.is_none())
            })
            .await
            .unwrap();
        assert!(matches!(
            publication
                .admit_refresh(&db, "a", true, Instant::now(), start, |_| {})
                .await
                .unwrap_err()
                .downcast_ref::<RefreshAccountError>(),
            Some(RefreshAccountError::AlreadyRefreshing)
        ));
        let _ = publication
            .admit_refresh(&db, "b", true, Instant::now(), start, |_| {})
            .await
            .unwrap();
        *publication
            .gate
            .lock()
            .await
            .refreshing
            .get("a")
            .unwrap()
            .status
            .write()
            .await = EnrollmentStatus::Failed(Err(anyhow!("synthetic 503")));
        publication
            .complete_refresh(&db, &ticket, &op, |_| {})
            .await
            .unwrap();
        assert!(matches!(
            publication
                .admit_refresh(&db, "a", true, Instant::now(), start, |_| {})
                .await
                .unwrap_err()
                .downcast_ref::<RefreshAccountError>(),
            Some(RefreshAccountError::Backoff)
        ));
        assert!(
            publication
                .admit_refresh(&db, "a", false, Instant::now(), start, |_| {})
                .await
                .is_ok()
        );
        publication.delete(&db, "a").await.unwrap();
        assert!(!publication.gate.lock().await.refreshing.contains_key("a"));
        assert!(
            publication
                .gate
                .lock()
                .await
                .schedule
                .eligible("a", Instant::now())
        );
        assert!(matches!(
            publication
                .admit_refresh(&db, "a", false, Instant::now(), start, |_| {})
                .await
                .unwrap_err()
                .downcast_ref::<RefreshAccountError>(),
            Some(RefreshAccountError::NoAccount)
        ));
    }
    #[tokio::test]
    async fn late_gdl_exchange_cannot_publish_after_replacement_or_credential_rotation() {
        let (_dir, db) = database().await;
        let publication = Publication::default();
        publication
            .save(&db, account("a", "old"), |_| {})
            .await
            .unwrap();
        let ticket = publication.gate.lock().await.capture("a");
        assert!(
            publication
                .publish_gdl_token(&db, &ticket, "dummy-id", "dummy-gdl", |args| assert!(
                    args.is_none()
                ))
                .await
                .unwrap()
        );
        publication.delete(&db, "a").await.unwrap();
        publication
            .save(&db, account("a", "new"), |_| {})
            .await
            .unwrap();
        assert!(
            !publication
                .publish_gdl_token(&db, &ticket, "dummy-id", "old-gdl", |_| panic!(
                    "stale exchange event"
                ))
                .await
                .unwrap()
        );
        assert!(
            account_repo::get_account(&db, "a")
                .await
                .unwrap()
                .unwrap()
                .gdl_token
                .is_none()
        );
        let ticket = publication.gate.lock().await.capture("a");
        assert!(
            !publication
                .publish_gdl_token(&db, &ticket, "previous-id", "old-gdl", |_| {})
                .await
                .unwrap()
        );
    }
}
