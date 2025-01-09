use crate::api::keys::account::*;
use crate::api::router::router;
use crate::domain::account as domain;
use crate::error::{AxumError, FeError};
use crate::managers::account::api::XboxError;
use crate::managers::account::gdl_account::{
    GDLAccountStatus, GDLUser, RegisterAccountBody, RequestGDLAccountDeletionError,
    RequestNewEmailChangeError, RequestNewVerificationTokenError,
};
use crate::managers::{account, App, AppInner};
use axum::extract::{Query, State};
use chrono::{DateTime, Utc};
use hyper::{header, StatusCode};
use rspc::RouterBuilder;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::Arc;

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

        query PEEK_GDL_ACCOUNT[app, uuid: String] {
            let gdl_user = app.account_manager().peek_gdl_account(uuid).await?;

            Ok(gdl_user.map(Into::<FEGDLAccount>::into))
        }

        query GET_GDL_ACCOUNT[app, args: ()] {
            let gdl_user = app.account_manager().get_gdl_account().await?;

            Ok(Into::<FEGDLAccountStatus>::into(gdl_user))
        }

        mutation REGISTER_GDL_ACCOUNT[app, register_data: FERegisterAccount] {
            let gdl_user = app.account_manager()
                .register_gdl_account(register_data.uuid.clone(), register_data.into())
                .await?;

            Ok(Into::<FEGDLAccount>::into(gdl_user))
        }

        mutation REQUEST_NEW_VERIFICATION_TOKEN[app, uuid: String] {
            let result = app.account_manager()
                .request_new_verification_token(uuid)
                .await;

            Ok(FERequestNewVerificationTokenStatus::from(result))
        }

        mutation REMOVE_GDL_ACCOUNT[app, _args: ()] {
            app.account_manager()
                .remove_gdl_account()
                .await
        }

        mutation SAVE_GDL_ACCOUNT[app, args: Option<String>] {
            app.account_manager()
                .save_gdl_account(args)
                .await
        }

        mutation REQUEST_EMAIL_CHANGE[app, args: FERequestEmailChange] {
            let result = app.account_manager()
                .request_email_change(args.uuid, args.email)
                .await;

            Ok(FERequestNewEmailChangeStatus::from(result))
        }

        mutation REQUEST_GDL_ACCOUNT_DELETION[app, uuid: String] {
            let result = app.account_manager()
                .request_gdl_account_deletion(uuid)
                .await;

            Ok(FERequestDeletionStatus::from(result))
        }

        mutation CHANGE_GDL_ACCOUNT_NICKNAME[app, args: FEChangeGdlAccountNickname] {
            app.account_manager()
                .change_nickname(args.uuid, args.nickname)
                .await
        }
    }
}

pub(super) fn mount_axum_router() -> axum::Router<Arc<AppInner>> {
    #[derive(Deserialize)]
    struct HeadQuery {
        uuid: String,
    }

    #[derive(Deserialize)]
    struct WaitForVerificationQuery {
        uuid: String,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct GdlUserAvatarQuery {
        user_id: String,
    }

    axum::Router::new()
        .route(
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
        .route(
            "/awaitForAccountVerification",
            axum::routing::get(
                |State(app): State<Arc<AppInner>>,
                 Query(query): Query<WaitForVerificationQuery>| async move {
                    app.account_manager()
                        .wait_for_account_verification(query.uuid.clone())
                        .await
                        .map_err(|e| FeError::from_anyhow(&e).make_axum())?;

                    app.invalidate(PEEK_GDL_ACCOUNT, Some(query.uuid.clone().into()));
                    app.invalidate(GET_GDL_ACCOUNT, None);

                    Ok::<_, AxumError>("ok".to_string())
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
    status: AccountStatus,
}

#[derive(Type, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type", content = "value")]
enum AccountType {
    Microsoft { email: Option<String> },
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
    RefreshingMSAuth,
    RequestingCode,
    PollingCode(DeviceCode),
    McLogin,
    XboxAuth,
    MCEntitlements,
    McProfile,
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

impl From<domain::AccountWithStatus> for AccountEntry {
    fn from(value: domain::AccountWithStatus) -> Self {
        Self {
            username: value.account.username,
            uuid: value.account.uuid,
            type_: value.account.type_.into(),
            last_used: value.account.last_used,
            status: value.status.into(),
        }
    }
}

impl From<domain::AccountType> for AccountType {
    fn from(value: domain::AccountType) -> Self {
        match value {
            domain::AccountType::Microsoft { email } => Self::Microsoft { email },
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
            BE::RefreshingMSAuth => Api::RefreshingMSAuth,
            BE::RequestingCode => Api::RequestingCode,
            BE::PollingCode(code) => Api::PollingCode(code.clone().into()),
            &BE::McLogin => Api::McLogin,
            &BE::XboxAuth => Api::XboxAuth,
            &BE::MCEntitlements => Api::MCEntitlements,
            &BE::McProfile => Api::McProfile,
            BE::Complete(account) => Api::Complete({
                // this is bad, but it used to be far worse
                let account: account::FullAccount = account.clone().into();
                let account: domain::AccountWithStatus = account.into();
                account.into()
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

#[derive(Type, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "status", content = "value")]
pub enum FEGDLAccountStatus {
    Valid(FEGDLAccount),
    Invalid,
    Skipped,
    Unset,
}

impl From<GDLAccountStatus> for FEGDLAccountStatus {
    fn from(value: GDLAccountStatus) -> Self {
        match value {
            GDLAccountStatus::Valid(value) => Self::Valid(value.into()),
            GDLAccountStatus::Invalid => Self::Invalid,
            GDLAccountStatus::Skipped => Self::Skipped,
            GDLAccountStatus::Unset => Self::Unset,
        }
    }
}

#[derive(Type, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FEGDLAccount {
    email: String,
    microsoft_oid: String,
    nickname: String,
    friend_code: String,
    profile_icon_url: String,
    microsoft_email: Option<String>,
    is_email_verified: bool,
    has_pending_verification: bool,
    verification_timeout: Option<u32>,
    has_pending_deletion_request: bool,
    deletion_timeout: Option<u32>,
    email_change_timeout: Option<u32>,
}

impl From<GDLUser> for FEGDLAccount {
    fn from(value: GDLUser) -> Self {
        Self {
            email: value.email,
            microsoft_oid: value.microsoft_oid,
            nickname: value.nickname,
            friend_code: value.friend_code,
            profile_icon_url: value.profile_icon_url,
            microsoft_email: value.microsoft_email,
            is_email_verified: value.is_verified,
            has_pending_verification: value.has_pending_verification,
            verification_timeout: value.verification_timeout.map(|v| v as u32),
            has_pending_deletion_request: value.has_pending_deletion_request,
            deletion_timeout: value.deletion_timeout.map(|v| v as u32),
            email_change_timeout: value.email_change_timeout.map(|v| v as u32),
        }
    }
}

#[derive(Type, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FERegisterAccount {
    pub email: String,
    pub nickname: String,
    pub uuid: String,
}

impl From<FERegisterAccount> for RegisterAccountBody {
    fn from(value: FERegisterAccount) -> Self {
        Self {
            email: value.email,
            nickname: value.nickname,
        }
    }
}

#[derive(Type, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "status", content = "value")]
pub enum FERequestNewVerificationTokenStatus {
    Success,
    Failed(Option<u32>),
}

impl From<Result<(), RequestNewVerificationTokenError>> for FERequestNewVerificationTokenStatus {
    fn from(value: Result<(), RequestNewVerificationTokenError>) -> Self {
        match value {
            Ok(_) => Self::Success,
            Err(RequestNewVerificationTokenError::TooManyRequests(cooldown)) => {
                Self::Failed(Some(cooldown))
            }
            Err(RequestNewVerificationTokenError::RequestFailed(_)) => Self::Failed(None),
        }
    }
}

#[derive(Type, Debug, Deserialize)]
pub struct FERequestEmailChange {
    pub email: String,
    pub uuid: String,
}

#[derive(Type, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "status", content = "value")]
pub enum FERequestNewEmailChangeStatus {
    Success,
    Failed(Option<u32>),
}

impl From<Result<(), RequestNewEmailChangeError>> for FERequestNewEmailChangeStatus {
    fn from(value: Result<(), RequestNewEmailChangeError>) -> Self {
        match value {
            Ok(_) => Self::Success,
            Err(RequestNewEmailChangeError::TooManyRequests(cooldown)) => {
                Self::Failed(Some(cooldown))
            }
            Err(RequestNewEmailChangeError::RequestFailed(_)) => Self::Failed(None),
        }
    }
}

#[derive(Type, Debug, Serialize)]
#[serde(rename_all = "camelCase")]

pub enum FERequestDeletionStatus {
    Success,
    Failed(Option<u32>),
}

impl From<Result<(), RequestGDLAccountDeletionError>> for FERequestDeletionStatus {
    fn from(value: Result<(), RequestGDLAccountDeletionError>) -> Self {
        match value {
            Ok(_) => Self::Success,
            Err(RequestGDLAccountDeletionError::TooManyRequests(cooldown)) => {
                Self::Failed(Some(cooldown))
            }
            Err(RequestGDLAccountDeletionError::RequestFailed(_)) => Self::Failed(None),
        }
    }
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FEChangeGdlAccountNickname {
    pub uuid: String,
    pub nickname: String,
}
