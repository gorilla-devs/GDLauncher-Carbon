use crate::api::keys::{self, account::*};
use crate::api::router::router;
use crate::domain::account as domain;
use crate::error::{AxumError, FeError};
use crate::managers::account::api::{UsernameAvailability, XboxError};
use crate::managers::account::gdl_account::{
    ChangeNicknameError, GDLAccountStatus, GDLUser, NicknameHistoryEntry, RegisterAccountBody,
    RequestGDLAccountDeletionError, RequestNewEmailChangeError, RequestNewVerificationTokenError,
};
use crate::managers::{App, AppInner, account};
use axum::extract::{Query, State};
use chrono::{DateTime, Utc};
use hyper::{StatusCode, header};
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

        mutation ENROLL_BEGIN_BROWSER[app, open_browser: bool] {
            app.account_manager().begin_enrollment_browser(open_browser).await
        }

        mutation ENROLL_PROTOCOL_CALLBACK[app, protocol_url: String] {
            app.account_manager().handle_protocol_callback(protocol_url).await
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

        mutation ENROLL_RESUME[app, args: ()] {
            app.account_manager().resume_enrollment().await
        }

        mutation REFRESH_ACCOUNT[app, uuid: String] {
            let _ = app.account_manager().refresh_account(uuid).await;

            Ok(())
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
            let result = app.account_manager()
                .change_nickname(args.uuid, args.nickname)
                .await;

            Ok(FEChangeNicknameStatus::from(result))
        }

        mutation UPLOAD_PROFILE_ICON[app, args: FEUploadProfileIcon] {
            app.account_manager().upload_profile_icon(args.uuid, args.icon_path).await
        }

        mutation DELETE_PROFILE_ICON[app, uuid: String] {
            app.account_manager().delete_profile_icon(uuid).await
        }

        mutation CHECK_USERNAME_AVAILABLE[app, args: FECheckUsernameAvailability] {
            app.account_manager()
                .check_username_available(args.access_token, args.username)
                .await
        }

        mutation CREATE_PROFILE[app, args: FECreateProfile] {
            app.account_manager()
                .create_minecraft_profile(args.access_token, args.username)
                .await
        }

        query GET_NICKNAME_HISTORY[app, friend_code: String] {
            let history = app.account_manager()
                .get_nickname_history(friend_code)
                .await?;

            Ok(history.into_iter().map(FENicknameHistoryEntry::from).collect::<Vec<_>>())
        }

        mutation CLEAR_NICKNAME_HISTORY[app, uuid: String] {
            app.account_manager()
                .clear_nickname_history(uuid)
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
    WaitingForBrowser {
        auth_url: String,
        redirect_uri: String,
        expires_at: DateTime<Utc>,
    },
    McLogin,
    XboxAuth,
    MCEntitlements,
    McProfile,
    NeedsProfileCreation {
        access_token: String,
    },
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
struct EnrollmentError {
    /// Short error identifier for programmatic handling
    error_type: EnrollmentErrorType,
    /// User-friendly error title
    title: String,
    /// Detailed error description
    description: String,
    /// List of suggested recovery steps
    recovery_steps: Vec<String>,
    /// Link to support documentation
    support_link: String,
    /// Original Xbox error if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    xbox_error: Option<XboxError>,
}

#[derive(Type, Serialize)]
#[serde(rename_all = "camelCase")]
enum EnrollmentErrorType {
    DeviceCodeExpired,
    /// signing in with xbox has returned an error
    XboxAccount,
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
        use EnrollmentStatus as Api;
        use account::EnrollmentStatus as BE;

        Ok(match value {
            BE::RefreshingMSAuth => Api::RefreshingMSAuth,
            BE::RequestingCode => Api::RequestingCode,
            BE::PollingCode(code) => Api::PollingCode(code.clone().into()),
            BE::WaitingForBrowser {
                auth_url,
                redirect_uri,
                expires_at,
            } => Api::WaitingForBrowser {
                auth_url: auth_url.clone(),
                redirect_uri: redirect_uri.clone(),
                expires_at: *expires_at,
            },
            &BE::McLogin => Api::McLogin,
            &BE::XboxAuth => Api::XboxAuth,
            &BE::MCEntitlements => Api::MCEntitlements,
            &BE::McProfile => Api::McProfile,
            BE::NeedsProfileCreation { access_token, .. } => Api::NeedsProfileCreation {
                access_token: access_token.clone(),
            },
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
            BE::DeviceCodeExpired => Self {
                error_type: EnrollmentErrorType::DeviceCodeExpired,
                title: value.title().to_string(),
                description: value.description(),
                recovery_steps: value.recovery_steps(),
                support_link: value.support_link().to_string(),
                xbox_error: None,
            },
            BE::XboxError(e) => Self {
                error_type: EnrollmentErrorType::XboxAccount,
                title: value.title().to_string(),
                description: value.description(),
                recovery_steps: value.recovery_steps(),
                support_link: value.support_link().to_string(),
                xbox_error: Some(e),
            },
            BE::EntitlementMissing => Self {
                error_type: EnrollmentErrorType::NoGameOwnership,
                title: value.title().to_string(),
                description: value.description(),
                recovery_steps: value.recovery_steps(),
                support_link: value.support_link().to_string(),
                xbox_error: None,
            },
            BE::GameProfileMissing => Self {
                error_type: EnrollmentErrorType::NoGameProfile,
                title: value.title().to_string(),
                description: value.description(),
                recovery_steps: value.recovery_steps(),
                support_link: value.support_link().to_string(),
                xbox_error: None,
            },
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
    has_custom_avatar: bool,
    microsoft_email: Option<String>,
    is_email_verified: bool,
    has_pending_verification: bool,
    verification_timeout: Option<u32>,
    has_pending_deletion_request: bool,
    deletion_timeout: Option<u32>,
    email_change_timeout: Option<u32>,
    nickname_change_timeout: Option<u32>,
}

impl From<GDLUser> for FEGDLAccount {
    fn from(value: GDLUser) -> Self {
        Self {
            email: value.email,
            microsoft_oid: value.microsoft_oid,
            nickname: value.nickname,
            friend_code: value.friend_code,
            profile_icon_url: value.profile_icon_url,
            has_custom_avatar: value.has_custom_avatar,
            microsoft_email: value.microsoft_email,
            is_email_verified: value.is_verified,
            has_pending_verification: value.has_pending_verification,
            verification_timeout: value.verification_timeout.map(|v| v as u32),
            has_pending_deletion_request: value.has_pending_deletion_request,
            deletion_timeout: value.deletion_timeout.map(|v| v as u32),
            email_change_timeout: value.email_change_timeout.map(|v| v as u32),
            nickname_change_timeout: value.nickname_change_timeout.map(|v| v as u32),
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

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FEUploadProfileIcon {
    pub uuid: String,
    pub icon_path: String,
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FECheckUsernameAvailability {
    pub access_token: String,
    pub username: String,
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FECreateProfile {
    pub access_token: String,
    pub username: String,
}

#[derive(Type, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "status", content = "value")]
pub enum FEChangeNicknameStatus {
    Success,
    Failed(Option<u32>),
}

impl From<Result<(), ChangeNicknameError>> for FEChangeNicknameStatus {
    fn from(value: Result<(), ChangeNicknameError>) -> Self {
        match value {
            Ok(_) => Self::Success,
            Err(ChangeNicknameError::TooManyRequests(cooldown)) => Self::Failed(Some(cooldown)),
            Err(ChangeNicknameError::RequestFailed(_)) => Self::Failed(None),
        }
    }
}

#[derive(Type, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FENicknameHistoryEntry {
    pub nickname: String,
    pub changed_at: DateTime<Utc>,
}

impl From<NicknameHistoryEntry> for FENicknameHistoryEntry {
    fn from(value: NicknameHistoryEntry) -> Self {
        Self {
            nickname: value.nickname,
            changed_at: value.changed_at,
        }
    }
}
