use std::sync::Arc;

use axum::extract::{Query, State};
use chrono::{DateTime, Utc};
use rspc::{RouterBuilder};
use specta::Type;
use serde::{Deserialize, Serialize};

use crate::api::keys::account::*;
use crate::api::router::router;
use crate::domain::account as domain;
use crate::error::FeError;
use crate::managers::account::api::XboxError;
use crate::managers::{account, App, AppInner};

pub(super) fn mount() -> RouterBuilder<App> {
    router! {
        query GET_ACTIVE_UUID[app, args: ()] {
            app.account_manager().get_active_uuid().await
        }

        mutation SET_ACTIVE_UUID[app, uuid: Option<String>] {
            app.account_manager().set_active_uuid(uuid).await
        }

        query GET_ACCOUNTS[app, args: ()] {
            Ok(app.account_manager()
               .get_account_list()
               .await?
               .into_iter()
               .map(AccountEntry::from)
               .collect::<Vec<_>>())
        }

        query GET_ACCOUNT_STATUS[app, uuid: String] {
            Ok(app.account_manager().get_account_status(uuid).await?
                .map(AccountStatus::from))
        }

        mutation DELETE_ACCOUNT[app, uuid: String] {
            app.account_manager().delete_account(uuid).await
        }

        mutation ENROLL_BEGIN[app, args: ()] {
            app.account_manager().begin_enrollment().await
        }

        mutation ENROLL_CANCEL[app, args: ()] {
            app.account_manager().cancel_enrollment().await
        }

        query(*) ENROLL_GET_STATUS[app, args: ()] {
            let r = app.account_manager().get_enrollment_status(|s| Result::<EnrollmentStatus, FeError>::from(s)).await;

            match r {
                None => Ok(None),
                Some(Ok(r)) => Ok(Some(r)),
                Some(Err(r)) => Err(r),
            }
        }

        mutation ENROLL_FINALIZE[app, args: ()] {
            app.account_manager().finalize_enrollment().await
        }

        mutation REFRESH_ACCOUNT[app, uuid: String] {
            app.account_manager().refresh_account(uuid).await
        }

        query GET_HEAD[_, _uuid: String] { Ok(()) }
    }
}

pub(super) fn mount_axum_router() -> axum::Router<Arc<AppInner>> {
    #[derive(Deserialize)]
    struct HeadQuery {
        uuid: String,
    }

    axum::Router::new().route(
        "/headImage",
        axum::routing::get(
            |State(app): State<Arc<AppInner>>, Query(query): Query<HeadQuery>| async move {
                app.account_manager()
                    .skin_manager()
                    .make_head(query.uuid)
                    .await
                    .map_err(|e| FeError::from_anyhow(&e).make_axum())
            },
        ),
    )
}

#[derive(Type, Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountEntry {
    username: String,
    uuid: String,
    last_used: DateTime<Utc>,
    type_: AccountType,
}

#[derive(Type, Serialize)]
#[serde(rename_all = "camelCase")]
enum AccountType {
    Microsoft,
    Offline,
}

#[derive(Type, Serialize)]
#[serde(rename_all = "camelCase")]
enum AccountStatus {
    Ok,
    Expired,
    Refreshing,
    Invalid,
}

#[derive(Type, Serialize)]
#[serde(rename_all = "camelCase")]
struct StatusFlags {
    banned_from_multiplayer: bool,
    xbox_disabled_multiplayer: bool,
}

#[derive(Type, Serialize)]
#[serde(rename_all = "camelCase")]
enum EnrollmentStatus {
    RequestingCode,
    PollingCode(DeviceCode),
    QueryingAccount,
    Complete(AccountEntry),
    Failed(EnrollmentError),
}

#[derive(Type, Serialize)]
#[serde(rename_all = "camelCase")]
struct DeviceCode {
    user_code: String,
    verification_uri: String,
    expires_at: DateTime<Utc>,
}

#[derive(Type, Serialize)]
#[serde(rename_all = "camelCase")]
enum EnrollmentError {
    DeviceCodeExpired,
    /// signing in with xbox has returned an error
    XboxAccount(XboxError),
    /// the user does not own the game OR is using xbox gamepass (this is not checked yet)
    NoGameOwnership,
    /// the user needs to log in once on the offical mc launcher
    NoGameProfile,
}

impl From<domain::Account> for AccountEntry {
    fn from(value: domain::Account) -> Self {
        Self {
            username: value.username,
            uuid: value.uuid,
            type_: value.type_.into(),
            last_used: value.last_used,
        }
    }
}

impl From<domain::AccountType> for AccountType {
    fn from(value: domain::AccountType) -> Self {
        match value {
            domain::AccountType::Microsoft => Self::Microsoft,
            domain::AccountType::Offline => Self::Offline,
        }
    }
}

impl From<domain::AccountStatus> for AccountStatus {
    fn from(value: domain::AccountStatus) -> Self {
        match value {
            domain::AccountStatus::Ok { access_token: _ } => Self::Ok,
            domain::AccountStatus::Refreshing => Self::Refreshing,
            domain::AccountStatus::Expired => Self::Expired,
            domain::AccountStatus::Invalid => Self::Invalid,
        }
    }
}

impl From<Option<domain::StatusFlags>> for StatusFlags {
    fn from(value: Option<domain::StatusFlags>) -> Self {
        match value {
            Some(domain::StatusFlags::BannedFromMultiplayer) => Self {
                banned_from_multiplayer: true,
                xbox_disabled_multiplayer: false,
            },
            Some(domain::StatusFlags::XboxMultiplayerDisabled) => Self {
                banned_from_multiplayer: false,
                xbox_disabled_multiplayer: true,
            },
            None => Self {
                banned_from_multiplayer: false,
                xbox_disabled_multiplayer: false,
            },
        }
    }
}

impl From<&account::EnrollmentStatus> for Result<EnrollmentStatus, FeError> {
    fn from(value: &account::EnrollmentStatus) -> Self {
        use account::EnrollmentStatus as BE;
        use EnrollmentStatus as Api;

        Ok(match value {
            BE::RequestingCode => Api::RequestingCode,
            BE::PollingCode(code) => Api::PollingCode(code.clone().into()),
            BE::McLogin | BE::PopulateAccount => Api::QueryingAccount,
            BE::Complete(account) => Api::Complete({
                // this is bad, but it used to be far worse
                let account: account::FullAccount = account.clone().into();
                let account: domain::AccountWithStatus = account.into();
                account.account.into()
            }),
            BE::Failed(e) => Api::Failed(EnrollmentError::from(
                e.as_ref().map_err(FeError::from_anyhow)?.clone(),
            )),
        })
    }
}

impl From<account::api::DeviceCode> for DeviceCode {
    fn from(value: account::api::DeviceCode) -> Self {
        Self {
            user_code: value.user_code,
            verification_uri: value.verification_uri,
            expires_at: value.expires_at,
        }
    }
}

impl From<account::EnrollmentError> for EnrollmentError {
    fn from(value: account::EnrollmentError) -> Self {
        use account::EnrollmentError as BE;

        match value {
            BE::DeviceCodeExpired => Self::DeviceCodeExpired,
            BE::XboxError(e) => Self::XboxAccount(e),
            BE::EntitlementMissing => Self::NoGameOwnership,
            BE::GameProfileMissing => Self::NoGameProfile,
        }
    }
}
