//! Profile verification continuation. Lock order: active enrollment → status, then
//! (without a status guard) bounded account publication. Provider work is outside
//! both locks; the active read guard linearizes local publication against Cancel.
use super::{EnrollmentStatus, EnrollmentTask, ResumeEnrollmentError, api};
use anyhow::{anyhow, bail};
use std::future::Future;
use tokio::sync::RwLock;

#[derive(serde::Serialize, specta::Type, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ProfileVerificationOutcome {
    Complete,
    Retryable,
    LoginRequired,
}

pub(super) async fn resume<V, VF, P, PF>(
    active: &RwLock<Option<EnrollmentTask>>,
    expected: &str,
    verify: V,
    publish: P,
    invalidate: impl Fn(),
) -> anyhow::Result<ProfileVerificationOutcome>
where
    V: FnOnce(String) -> VF,
    VF: Future<Output = anyhow::Result<Result<api::McProfile, api::GetProfileError>>>,
    P: FnOnce(api::FullAccount) -> PF,
    PF: Future<Output = anyhow::Result<()>>,
{
    let enrollment_opt = active.read().await;
    let enrollment = enrollment_opt
        .as_ref()
        .ok_or(ResumeEnrollmentError::NotActive)?;
    if enrollment.id != expected {
        bail!(ResumeEnrollmentError::Obsolete);
    }
    let cancel = enrollment.cancel.clone();
    let mut status_lock = enrollment.status.write().await;
    let status = std::mem::replace(&mut *status_lock, EnrollmentStatus::McProfile);
    let EnrollmentStatus::NeedsProfileCreation {
        access_token,
        ms_auth,
        mc_auth,
        entitlements,
        ..
    } = status
    else {
        *status_lock = status;
        bail!(ResumeEnrollmentError::NotInProfileCreationState);
    };
    drop(status_lock);
    drop(enrollment_opt);
    invalidate();
    let result = tokio::select! {
        biased;
        _ = cancel.cancelled() => return Err(ResumeEnrollmentError::Obsolete.into()),
        result = verify(access_token.clone()) => result,
    };
    let enrollment_opt = active.read().await;
    let enrollment = enrollment_opt
        .as_ref()
        .filter(|e| e.id == expected && !e.cancel.is_cancelled())
        .ok_or(ResumeEnrollmentError::Obsolete)?;
    let retry = || EnrollmentStatus::NeedsProfileCreation {
        access_token: access_token.clone(),
        verification_retry: true,
        ms_auth: ms_auth.clone(),
        mc_auth: mc_auth.clone(),
        entitlements: entitlements.clone(),
    };
    let profile = match result {
        Ok(Ok(profile)) => profile,
        Ok(Err(api::GetProfileError::AuthTokenInvalid)) => {
            *enrollment.status.write().await =
                EnrollmentStatus::Failed(Err(api::GetProfileError::AuthTokenInvalid.into()));
            invalidate();
            return Ok(ProfileVerificationOutcome::LoginRequired);
        }
        // Includes propagation 404, connectivity, throttling, server and parse failures.
        _ => {
            *enrollment.status.write().await = retry();
            invalidate();
            return Ok(ProfileVerificationOutcome::Retryable);
        }
    };
    let account = api::FullAccount {
        ms: ms_auth.clone(),
        mc: api::McAccount {
            entitlement: entitlements.clone(),
            profile,
            auth: mc_auth.clone(),
        },
        gdl_token: None,
    };
    if let Err(error) = publish(account.clone()).await {
        *enrollment.status.write().await = retry();
        invalidate();
        return Err(error);
    }
    *enrollment.status.write().await = EnrollmentStatus::Complete(account);
    invalidate();
    Ok(ProfileVerificationOutcome::Complete)
}

#[cfg(test)]
mod task19_tests {
    use super::*;
    use chrono::Utc;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use std::time::Duration;

    fn needs_profile() -> EnrollmentStatus {
        EnrollmentStatus::NeedsProfileCreation {
            access_token: "dummy-mc".into(),
            verification_retry: false,
            ms_auth: api::MsAuth {
                access_token: "dummy-ms".into(),
                id_token: "dummy-id".into(),
                refresh_token: "dummy-refresh".into(),
                expires_at: Utc::now(),
            },
            mc_auth: api::McAuth {
                access_token: "dummy-mc".into(),
                expires_at: Utc::now(),
            },
            entitlements: api::McEntitlement::Owned,
        }
    }
    fn profile() -> api::McProfile {
        api::McProfile {
            uuid: "synthetic-a".into(),
            username: "Example".into(),
            skin: None,
        }
    }
    async fn verify(_: String) -> anyhow::Result<Result<api::McProfile, api::GetProfileError>> {
        Ok(Ok(profile()))
    }

    #[tokio::test]
    async fn wrong_state_resume_releases_guards() {
        let active = Arc::new(RwLock::new(Some(EnrollmentTask::inert(
            EnrollmentStatus::McProfile,
        ))));
        let id = active.read().await.as_ref().unwrap().id.clone();
        let operation = {
            let active = active.clone();
            tokio::spawn(
                async move { resume(&active, &id, verify, |_| async { Ok(()) }, || {}).await },
            )
        };
        let mut operation = operation;
        let result = tokio::time::timeout(Duration::from_millis(100), &mut operation).await;
        if result.is_err() {
            assert!(
                tokio::time::timeout(Duration::from_millis(30), active.write())
                    .await
                    .is_err(),
                "original call must remain alive for blocked-lock evidence"
            );
        }
        assert!(
            result.is_ok(),
            "wrong-state resume deadlocked while retaining both guards"
        );
        assert!(
            result
                .unwrap()
                .unwrap()
                .unwrap_err()
                .is::<ResumeEnrollmentError>()
        );
        assert!(matches!(
            *active.read().await.as_ref().unwrap().status.read().await,
            EnrollmentStatus::McProfile
        ));
    }

    #[tokio::test]
    async fn cancelled_resume_never_publishes_into_replacement() {
        let active = Arc::new(RwLock::new(Some(EnrollmentTask::inert(needs_profile()))));
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (finish_tx, finish_rx) = tokio::sync::oneshot::channel();
        let published = Arc::new(AtomicUsize::new(0));
        let id = active.read().await.as_ref().unwrap().id.clone();
        let operation = {
            let active = active.clone();
            let published = published.clone();
            tokio::spawn(async move {
                resume(
                    &active,
                    &id,
                    |_| async {
                        started_tx.send(()).unwrap();
                        finish_rx.await.unwrap();
                        Ok(Ok(profile()))
                    },
                    |_| async {
                        published.fetch_add(1, Ordering::SeqCst);
                        Ok(())
                    },
                    || {},
                )
                .await
            })
        };
        started_rx.await.unwrap();
        *active.write().await = Some(EnrollmentTask::inert(EnrollmentStatus::RequestingCode));
        let _ = finish_tx.send(());
        let _ = operation.await.unwrap();
        assert_eq!(
            published.load(Ordering::SeqCst),
            0,
            "cancelled account was persisted/selected"
        );
        assert!(matches!(
            *active.read().await.as_ref().unwrap().status.read().await,
            EnrollmentStatus::RequestingCode
        ));
    }

    #[tokio::test]
    async fn temporary_failure_preserves_retry_context() {
        let active = RwLock::new(Some(EnrollmentTask::inert(needs_profile())));
        let id = active.read().await.as_ref().unwrap().id.clone();
        let error = resume(
            &active,
            &id,
            |_| async { Err(anyhow!("synthetic connectivity failure")) },
            |_| async { Ok(()) },
            || {},
        )
        .await;
        assert_eq!(error.unwrap(), ProfileVerificationOutcome::Retryable);
        assert!(
            matches!(
                *active.read().await.as_ref().unwrap().status.read().await,
                EnrollmentStatus::NeedsProfileCreation { .. }
            ),
            "verification destroyed authentication retry context"
        );
    }
    async fn persisted_resume(
        active: &RwLock<Option<EnrollmentTask>>,
        id: &str,
        db: &carbon_repos::db_exec::Db,
        publication: &super::super::publication::Publication,
    ) -> anyhow::Result<ProfileVerificationOutcome> {
        resume(
            active,
            id,
            verify,
            |account| async move {
                publication
                    .publish_enrollment(
                        db,
                        account.into(),
                        |uuid| async move {
                            let patch =
                                carbon_repos::repos::app_configuration::AppConfigurationPatch {
                                    active_account_uuid: Some(Some(uuid)),
                                    ..Default::default()
                                }
                                .build()
                                .unwrap();
                            db.write(move |conn| Ok(patch.execute(&conn)?)).await?;
                            Ok(())
                        },
                        |_| {},
                    )
                    .await?;
                Ok(())
            },
            || {},
        )
        .await
    }
    #[tokio::test]
    async fn retry_persists_full_authentication_and_selects_the_same_enrollment() {
        use super::super::publication::{Publication, task19_publication_tests::database};
        let (_dir, db) = database().await;
        carbon_repos::repos::app_configuration::insert_app_configuration(
            &db,
            "stable".into(),
            2048,
            None,
        )
        .await
        .unwrap();
        let publication = Publication::default();
        let active = RwLock::new(Some(EnrollmentTask::inert(needs_profile())));
        let id = active.read().await.as_ref().unwrap().id.clone();
        for failure in ["connectivity", "429", "503", "parse"] {
            let result = resume(
                &active,
                &id,
                |_| async { Err(anyhow!(failure.to_owned())) },
                |_| async { panic!("failure publication") },
                || {},
            )
            .await
            .unwrap();
            assert_eq!(result, ProfileVerificationOutcome::Retryable);
        }
        assert_eq!(
            persisted_resume(&active, &id, &db, &publication)
                .await
                .unwrap(),
            ProfileVerificationOutcome::Complete
        );
        let row = carbon_repos::repos::account::get_account(&db, "synthetic-a")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.ms_refresh_token.as_deref(), Some("dummy-refresh"));
        assert_eq!(row.id_token.as_deref(), Some("dummy-id"));
        assert_eq!(row.access_token.as_deref(), Some("dummy-mc"));
        let settings = carbon_repos::repos::app_configuration::get_app_configuration(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(settings.active_account_uuid.as_deref(), Some("synthetic-a"));
    }
    #[tokio::test]
    async fn auth_invalid_is_terminal_but_propagation_missing_is_retryable() {
        for invalid in [false, true] {
            let active = RwLock::new(Some(EnrollmentTask::inert(needs_profile())));
            let id = active.read().await.as_ref().unwrap().id.clone();
            let result = resume(
                &active,
                &id,
                |_| async {
                    Ok(Err(if invalid {
                        api::GetProfileError::AuthTokenInvalid
                    } else {
                        api::GetProfileError::GameProfileMissing
                    }))
                },
                |_| async { panic!("no account before verification") },
                || {},
            )
            .await
            .unwrap();
            assert_eq!(
                result,
                if invalid {
                    ProfileVerificationOutcome::LoginRequired
                } else {
                    ProfileVerificationOutcome::Retryable
                }
            );
        }
    }
    #[tokio::test]
    async fn every_unsupported_state_and_obsolete_flow_returns_promptly() {
        for status in [
            EnrollmentStatus::RequestingCode,
            EnrollmentStatus::McLogin,
            EnrollmentStatus::XboxAuth,
            EnrollmentStatus::MCEntitlements,
            EnrollmentStatus::McProfile,
            EnrollmentStatus::WaitingForBrowser {
                auth_url: "https://example.invalid".into(),
                redirect_uri: "https://example.invalid/callback".into(),
                expires_at: Utc::now(),
            },
            EnrollmentStatus::Complete(
                super::super::publication::task19_publication_tests::api_account("dummy"),
            ),
        ] {
            let before = std::mem::discriminant(&status);
            let active = RwLock::new(Some(EnrollmentTask::inert(status)));
            let id = active.read().await.as_ref().unwrap().id.clone();
            let result = tokio::time::timeout(
                Duration::from_millis(100),
                resume(
                    &active,
                    &id,
                    verify,
                    |_| async { panic!("wrong-state publication") },
                    || {},
                ),
            )
            .await
            .unwrap();
            assert!(matches!(
                result.unwrap_err().downcast_ref::<ResumeEnrollmentError>(),
                Some(ResumeEnrollmentError::NotInProfileCreationState)
            ));
            assert_eq!(
                std::mem::discriminant(&*active.read().await.as_ref().unwrap().status.read().await),
                before
            );
            active.write().await.take();
            let error = resume(&active, &id, verify, |_| async { Ok(()) }, || {})
                .await
                .unwrap_err();
            assert!(matches!(
                error.downcast_ref::<ResumeEnrollmentError>(),
                Some(ResumeEnrollmentError::NotActive)
            ));
            *active.write().await = Some(EnrollmentTask::inert(needs_profile()));
            assert!(matches!(
                resume(&active, &id, verify, |_| async { Ok(()) }, || {})
                    .await
                    .unwrap_err()
                    .downcast_ref::<ResumeEnrollmentError>(),
                Some(ResumeEnrollmentError::Obsolete)
            ));
        }
    }
    #[tokio::test]
    async fn final_publication_barrier_serializes_cancel_after_verification() {
        let active = Arc::new(RwLock::new(Some(EnrollmentTask::inert(needs_profile()))));
        let id = active.read().await.as_ref().unwrap().id.clone();
        let (entered, ready) = tokio::sync::oneshot::channel();
        let (release, released) = tokio::sync::oneshot::channel();
        let published = Arc::new(AtomicUsize::new(0));
        let operation = {
            let active = active.clone();
            let published = published.clone();
            tokio::spawn(async move {
                resume(
                    &active,
                    &id,
                    verify,
                    |_| async {
                        entered.send(()).unwrap();
                        released.await.unwrap();
                        published.fetch_add(1, Ordering::SeqCst);
                        Ok(())
                    },
                    || {},
                )
                .await
            })
        };
        ready.await.unwrap();
        // The response has already completed. Cancel must wait for this bounded
        // publication, not retire ownership between persistence and selection.
        let mut cancellation = Box::pin(active.write());
        assert!(
            tokio::time::timeout(Duration::from_millis(30), &mut cancellation)
                .await
                .is_err()
        );
        assert_eq!(published.load(Ordering::SeqCst), 0);
        release.send(()).unwrap();
        assert_eq!(
            operation.await.unwrap().unwrap(),
            ProfileVerificationOutcome::Complete
        );
        cancellation.await.take();
        assert_eq!(published.load(Ordering::SeqCst), 1);
        assert!(active.read().await.is_none());
    }

    #[tokio::test]
    async fn cancellation_during_verification_does_not_persist_or_select() {
        use super::super::publication::{Publication, task19_publication_tests::database};
        for replacement in [false, true] {
            let (_dir, db) = database().await;
            let db = Arc::new(db);
            let publication = Arc::new(Publication::default());
            carbon_repos::repos::app_configuration::insert_app_configuration(
                &db,
                "stable".into(),
                2048,
                None,
            )
            .await
            .unwrap();
            let active = Arc::new(RwLock::new(Some(EnrollmentTask::inert(needs_profile()))));
            let id = active.read().await.as_ref().unwrap().id.clone();
            let (entered, ready) = tokio::sync::oneshot::channel();
            let task = {
                let active = active.clone();
                let db = db.clone();
                let publication = publication.clone();
                tokio::spawn(async move {
                    resume(
                        &active,
                        &id,
                        |_| async {
                            entered.send(()).unwrap();
                            std::future::pending::<
                                anyhow::Result<Result<api::McProfile, api::GetProfileError>>,
                            >()
                            .await
                        },
                        |account| async { publication.save(&db, account.into(), |_| {}).await },
                        || {},
                    )
                    .await
                })
            };
            ready.await.unwrap();
            *active.write().await = if replacement {
                Some(EnrollmentTask::inert(EnrollmentStatus::RequestingCode))
            } else {
                None
            };
            assert!(task.await.unwrap().is_err());
            assert!(
                carbon_repos::repos::account::get_accounts_by_last_used(&db)
                    .await
                    .unwrap()
                    .is_empty()
            );
            assert!(
                carbon_repos::repos::app_configuration::get_app_configuration(&db)
                    .await
                    .unwrap()
                    .unwrap()
                    .active_account_uuid
                    .is_none()
            );
        }
    }
}
