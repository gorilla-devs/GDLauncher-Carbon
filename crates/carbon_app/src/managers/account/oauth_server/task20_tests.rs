use super::*;

fn fixture() -> (CallbackState, OAuthCallbackHandle) {
    let code = watch::channel(None).0;
    let state = CallbackState {
        code: code.clone(),
        expected_state: "synthetic-state".into(),
    };
    let handle = OAuthCallbackHandle {
        code,
        local_addr: "127.0.0.1:1".parse().unwrap(),
        server_task: tokio::spawn(async {}),
        _shutdown_tx: None,
    };
    (state, handle)
}
async fn callback(
    state: &CallbackState,
    supplied: Option<&str>,
    code: Option<&str>,
    error: Option<&str>,
) -> String {
    let response = oauth_callback(
        Query(OAuthCallbackQuery {
            state: supplied.map(str::to_owned),
            code: code.map(str::to_owned),
            error: error.map(str::to_owned),
            error_description: Some("Cancelled <by> user & choice".into()),
        }),
        State(state.clone()),
    )
    .await
    .into_response();
    String::from_utf8(
        axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap()
}
#[tokio::test]
async fn accepted_denial_wakes_actual_waiter_immediately() {
    let (state, handle) = fixture();
    let waiter = tokio::spawn(async move { handle.wait_for_code(Duration::from_secs(300)).await });
    tokio::task::yield_now().await;
    let page = callback(&state, Some("synthetic-state"), None, Some("access_denied")).await;
    assert!(page.contains("access_denied"));
    assert!(page.contains("Cancelled &lt;by&gt; user &amp; choice"));
    let result = timeout(Duration::from_millis(100), waiter)
        .await
        .expect("accepted denial did not wake waiter")
        .unwrap()
        .unwrap_err()
        .to_string();
    assert!(result.contains("access_denied") && result.contains("Cancelled <by> user & choice"));
}
#[tokio::test]
async fn early_denial_and_duplicate_callback_keep_first_terminal() {
    let (state, handle) = fixture();
    callback(&state, Some("synthetic-state"), None, Some("access_denied")).await;
    callback(&state, Some("synthetic-state"), Some("later-code"), None).await;
    let error = timeout(
        Duration::from_millis(100),
        handle.wait_for_code(Duration::from_secs(300)),
    )
    .await
    .unwrap()
    .unwrap_err()
    .to_string();
    assert!(error.contains("access_denied"));
}
#[tokio::test]
async fn wrong_missing_state_nonterminal_and_success_keeps_first_code() {
    let (state, handle) = fixture();
    for supplied in [None, Some("wrong")] {
        assert!(
            callback(&state, supplied, None, Some("access_denied"))
                .await
                .contains("Invalid state")
        );
        assert!(
            callback(&state, supplied, Some("wrong-state-code"), None)
                .await
                .contains("Invalid state")
        );
        assert!(
            timeout(
                Duration::from_millis(10),
                handle.wait_for_code(Duration::from_secs(300))
            )
            .await
            .is_err()
        );
    }
    callback(&state, Some("synthetic-state"), Some("first-code"), None).await;
    callback(&state, Some("synthetic-state"), None, Some("access_denied")).await;
    assert_eq!(
        handle
            .wait_for_code(Duration::from_secs(300))
            .await
            .unwrap(),
        "first-code"
    );
}

#[tokio::test]
async fn pending_success_wakes_and_duplicate_success_cannot_overwrite() {
    let (state, handle) = fixture();
    let waiter = tokio::spawn(async move { handle.wait_for_code(Duration::from_secs(300)).await });
    tokio::task::yield_now().await;
    callback(&state, Some("synthetic-state"), Some("first-code"), None).await;
    callback(&state, Some("synthetic-state"), Some("second-code"), None).await;
    assert_eq!(
        timeout(Duration::from_millis(100), waiter)
            .await
            .unwrap()
            .unwrap()
            .unwrap(),
        "first-code"
    );
    let (state, handle) = fixture();
    oauth_callback(
        Query(OAuthCallbackQuery {
            code: None,
            error: Some("access_denied".into()),
            error_description: None,
            state: Some("synthetic-state".into()),
        }),
        State(state),
    )
    .await;
    assert_eq!(
        handle
            .wait_for_code(Duration::from_secs(300))
            .await
            .unwrap_err()
            .to_string(),
        "access_denied: Unknown error"
    );
}
#[tokio::test]
async fn no_callback_timeout_and_actual_enrollment_drop_abort() {
    let (_, handle) = fixture();
    assert!(
        handle
            .wait_for_code(Duration::from_millis(10))
            .await
            .unwrap_err()
            .to_string()
            .contains("Timeout waiting")
    );
    let (_, handle) = fixture();
    let (enrollment, task) = super::super::enroll::EnrollmentTask::running_for_test(async move {
        let _ = handle.wait_for_code(Duration::from_secs(300)).await;
        panic!("cancelled enrollment continued");
    });
    let cancel = enrollment.cancel.clone();
    drop(enrollment);
    assert!(cancel.is_cancelled());
    assert!(
        timeout(Duration::from_millis(100), task)
            .await
            .unwrap()
            .unwrap()
            .is_err()
    );
}

#[tokio::test]
async fn accepted_denial_publishes_failure_and_never_exchanges_tokens() {
    use super::super::enroll::{
        EnrollmentStatus, InvalidateCtx, SignInMethod, exchange_browser_code, publish_result,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct Invalidation(AtomicUsize);
    #[async_trait::async_trait]
    impl InvalidateCtx for Invalidation {
        async fn invalidate(&self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    let (state, handle) = fixture();
    callback(&state, Some("synthetic-state"), None, Some("access_denied")).await;
    let exchanges = AtomicUsize::new(0);
    let result = exchange_browser_code(handle.wait_for_code(Duration::from_secs(300)), |_| async {
        exchanges.fetch_add(1, Ordering::SeqCst);
        panic!("provider denial reached token exchange");
    })
    .await;
    assert_eq!(exchanges.load(Ordering::SeqCst), 0);
    let status = tokio::sync::RwLock::new(EnrollmentStatus::RequestingCode);
    let invalidate = Invalidation(AtomicUsize::new(0));
    publish_result(
        &status,
        &invalidate,
        SignInMethod::Browser,
        result.map(|_| ()).map_err(Into::into),
    )
    .await;
    assert_eq!(invalidate.0.load(Ordering::SeqCst), 1);
    let current = status.read().await;
    let EnrollmentStatus::Failed(Err(error)) = &*current else {
        panic!("denial did not publish actual terminal failure")
    };
    assert!(error.is::<ProviderDenied>());
    assert!(error.to_string().contains("Cancelled <by> user & choice"));
}
