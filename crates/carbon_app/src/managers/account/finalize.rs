//! Bounded local finalization; optional provider work stays in the caller.
use super::{EnrollmentStatus, EnrollmentTask, FinalizeEnrollmentError, api};
use tokio::sync::RwLock;

#[derive(Debug, PartialEq, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum FinalizeEnrollmentOutcome {
    Complete,
    Obsolete,
}

pub(super) async fn finalize<T, F, Fut>(
    active: &RwLock<Option<EnrollmentTask>>,
    expected: &str,
    publish: F,
    invalidate: impl FnOnce(),
) -> anyhow::Result<Option<T>>
where
    F: FnOnce(api::FullAccount) -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<T>>,
{
    let mut active = active.write().await;
    let Some(enrollment) = active.as_ref() else {
        return Ok(None);
    };
    if enrollment.id != expected || enrollment.cancel.is_cancelled() {
        return Ok(None);
    }
    let account = match &*enrollment.status.read().await {
        EnrollmentStatus::Complete(account) => account.clone(),
        _ => return Err(FinalizeEnrollmentError::NotComplete.into()),
    };
    let result = publish(account).await?;
    active.take();
    invalidate();
    Ok(Some(result))
}

#[cfg(test)]
mod task19_finalize_tests {
    use super::super::publication::task19_publication_tests::api_account;
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn obsolete_finalize_request_preserves_replacement_and_has_no_effects() {
        let old = EnrollmentTask::inert(EnrollmentStatus::Complete(api_account("old")));
        let expected = old.id.clone();
        let replacement = EnrollmentTask::inert(EnrollmentStatus::Complete(api_account("new")));
        let replacement_id = replacement.id.clone();
        let active = RwLock::new(Some(replacement));
        let writes = AtomicUsize::new(0);
        let events = AtomicUsize::new(0);
        let result = finalize(
            &active,
            &expected,
            |_| async {
                writes.fetch_add(1, Ordering::SeqCst);
                Ok(())
            },
            || {
                events.fetch_add(1, Ordering::SeqCst);
            },
        )
        .await;
        assert!(
            result.unwrap().is_none(),
            "old Complete request finalized the replacement enrollment"
        );
        assert_eq!(writes.load(Ordering::SeqCst), 0);
        assert_eq!(events.load(Ordering::SeqCst), 0);
        assert_eq!(active.read().await.as_ref().unwrap().id, replacement_id);
    }
    #[tokio::test]
    async fn current_finalize_persists_selects_and_retires_once() {
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
        let active = RwLock::new(Some(EnrollmentTask::inert(EnrollmentStatus::Complete(
            api_account("current"),
        ))));
        let expected = active.read().await.as_ref().unwrap().id.clone();
        let events = AtomicUsize::new(0);
        let ticket = finalize(
            &active,
            &expected,
            |account| async {
                publication
                    .publish_enrollment(
                        &db,
                        account.into(),
                        |uuid| async {
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
                    .await
            },
            || {
                events.fetch_add(1, Ordering::SeqCst);
            },
        )
        .await
        .unwrap();
        assert_eq!(ticket.unwrap().uuid, "a");
        assert!(active.read().await.is_none());
        assert_eq!(events.load(Ordering::SeqCst), 1);
        assert_eq!(
            carbon_repos::repos::account::get_account(&db, "a")
                .await
                .unwrap()
                .unwrap()
                .access_token
                .as_deref(),
            Some("current")
        );
        assert_eq!(
            carbon_repos::repos::app_configuration::get_app_configuration(&db)
                .await
                .unwrap()
                .unwrap()
                .active_account_uuid
                .as_deref(),
            Some("a")
        );
    }

    #[tokio::test]
    async fn missing_and_incomplete_finalize_preserve_state_without_publication() {
        let active = RwLock::new(None);
        let result: anyhow::Result<Option<()>> = finalize(
            &active,
            "absent",
            |_| async { panic!("no publication") },
            || panic!("no event"),
        )
        .await;
        assert!(result.unwrap().is_none());
        *active.write().await = Some(EnrollmentTask::inert(EnrollmentStatus::McProfile));
        let expected = active.read().await.as_ref().unwrap().id.clone();
        let result: anyhow::Result<Option<()>> = finalize(
            &active,
            &expected,
            |_| async { panic!("no publication") },
            || panic!("no event"),
        )
        .await;
        assert!(matches!(
            result
                .unwrap_err()
                .downcast_ref::<FinalizeEnrollmentError>(),
            Some(FinalizeEnrollmentError::NotComplete)
        ));
        assert!(matches!(
            *active.read().await.as_ref().unwrap().status.read().await,
            EnrollmentStatus::McProfile
        ));
    }

    #[tokio::test]
    async fn current_finalize_holds_publication_against_cancel_and_preserves_failure() {
        use std::sync::Arc;
        let active = Arc::new(RwLock::new(Some(EnrollmentTask::inert(
            EnrollmentStatus::Complete(api_account("current")),
        ))));
        let expected = active.read().await.as_ref().unwrap().id.clone();
        let (entered, ready) = tokio::sync::oneshot::channel();
        let (release, released) = tokio::sync::oneshot::channel();
        let operation = {
            let active = active.clone();
            tokio::spawn(async move {
                finalize(
                    &active,
                    &expected,
                    |_| async {
                        entered.send(()).unwrap();
                        released.await.unwrap();
                        Err::<(), _>(anyhow::anyhow!("synthetic local write failure"))
                    },
                    || panic!("no success event"),
                )
                .await
            })
        };
        ready.await.unwrap();
        let mut cancel = Box::pin(active.write());
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(30), &mut cancel)
                .await
                .is_err()
        );
        release.send(()).unwrap();
        assert!(operation.await.unwrap().is_err());
        let mut owner = cancel.await;
        assert!(matches!(
            *owner.as_ref().unwrap().status.read().await,
            EnrollmentStatus::Complete(_)
        ));
        owner.take();
    }
}
