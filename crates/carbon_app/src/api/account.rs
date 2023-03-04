use chrono::{DateTime, Utc};
use rspc::{RouterBuilderLike, Type};
use serde::Serialize;

use crate::api::keys::account::*;
use crate::api::router::router;
use crate::error::into_rspc;
use crate::error::request::FERequestError;
use crate::managers::account::api::{
    DeviceCodePollError, DeviceCodeRequestError, McAccountPopulateError, McAuthError,
    McEntitlementCheckError, McEntitlementError, McProfileError, McProfileRequestError,
    XboxAuthError, XboxError,
};
use crate::managers::{account, App};
use carbon_domain::account as domain;

pub(super) fn mount() -> impl RouterBuilderLike<App> {
    router! {
        query GET_ACTIVE_UUID[app, _: ()] {
            app.account_manager().get_active_uuid().await
               .map_err(into_rspc)
        }

        mutation SET_ACTIVE_UUID[app, uuid: Option<String>] {
            app.account_manager().set_active_uuid(uuid).await
                .map_err(into_rspc)
        }

        query GET_ACCOUNTS[app, _: ()] {
            Ok(app.account_manager()
               .get_account_list()
               .await
               .map_err(into_rspc)?
               .into_iter()
               .map(AccountEntry::from)
               .collect::<Vec<_>>())
        }

        query GET_ACCOUNT_STATUS[app, uuid: String] {
            Ok(app.account_manager().get_account_status(uuid).await
                .map_err(into_rspc)?
                .map(AccountStatus::from))
        }

        mutation DELETE_ACCOUNT[app, uuid: String] {
            app.account_manager().delete_account(uuid).await
                .map_err(into_rspc)
        }

        mutation ENROLL_BEGIN[app, _: ()] {
            app.account_manager().begin_enrollment().await
                .map_err(into_rspc)
        }

        mutation ENROLL_CANCEL[app, _: ()] {
            app.account_manager().cancel_enrollment().await
                .map_err(into_rspc)
        }

        query ENROLL_GET_STATUS[app, _: ()] {
            Ok(EnrollmentStatus::from(
                app.account_manager().get_enrollment_status().await
                    .map_err(into_rspc)?
            ))
        }

        mutation ENROLL_FINALIZE[app, _: ()] {
            app.account_manager().finalize_enrollment().await
                .map_err(into_rspc)
        }
    }
}

#[derive(Type, Serialize)]
struct AccountEntry {
    username: String,
    uuid: String,
    type_: AccountType,
}

#[derive(Type, Serialize)]
enum AccountType {
    Microsoft,
    Offline,
}

#[derive(Type, Serialize)]
enum AccountStatus {
    Ok,
    Expired,
    Refreshing,
}

#[derive(Type, Serialize)]
enum EnrollmentStatus {
    RequestingCode,
    PollingCode(DeviceCode),
    QueryingAccount,
    Complete(AccountEntry),
    Failed(EnrollmentError),
}

#[derive(Type, Serialize)]
struct DeviceCode {
    user_code: String,
    verification_uri: String,
    expires_at: DateTime<Utc>,
}

#[derive(Type, Serialize)]
enum EnrollmentError {
    /// web request related error
    Request(FERequestError),
    DeviceCodeExpired,
    /// signing in with xbox has returned an error
    XboxAccount(XboxError),
    /// the account details response from the mojang server has likely been tampered with
    AccountSigningError,
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
            domain::AccountStatus::Ok { .. } => Self::Ok,
            domain::AccountStatus::Expired => Self::Expired,
            domain::AccountStatus::Refreshing => Self::Refreshing,
        }
    }
}

impl From<account::FEEnrollmentStatus> for EnrollmentStatus {
    fn from(value: account::FEEnrollmentStatus) -> Self {
        use account::FEEnrollmentStatus as Status;

        match value {
            Status::RequestingCode => Self::RequestingCode,
            Status::PollingCode(code) => Self::PollingCode(code.into()),
            Status::QueryAccount => Self::QueryingAccount,
            Status::Complete(account) => Self::Complete(account.into()),
            Status::Failed(msg) => Self::Failed(msg.into()),
        }
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
            BE::DeviceCodeRequest(DeviceCodeRequestError::Request(x))
            | BE::DeviceCodePoll(DeviceCodePollError::Request(x))
            | BE::McAuth(McAuthError::Request(x))
            | BE::McAuth(McAuthError::Xbox(XboxAuthError::Request(x)))
            | BE::AccountPopulate(McAccountPopulateError::Entitlement(
                McEntitlementCheckError::Request(x),
            ))
            | BE::AccountPopulate(McAccountPopulateError::Profile(
                McProfileRequestError::Request(x),
            )) => Self::Request(x.into()),

            BE::DeviceCodePoll(DeviceCodePollError::CodeExpired) => Self::DeviceCodeExpired,

            BE::McAuth(McAuthError::Xbox(XboxAuthError::Xbox(x))) => Self::XboxAccount(x),

            BE::AccountPopulate(McAccountPopulateError::Entitlement(
                McEntitlementCheckError::Entitlement(McEntitlementError::NoEntitlement),
            )) => Self::NoGameOwnership,

            BE::AccountPopulate(McAccountPopulateError::Entitlement(
                McEntitlementCheckError::Entitlement(_),
            )) => Self::AccountSigningError,

            BE::AccountPopulate(McAccountPopulateError::Profile(
                McProfileRequestError::Profile(McProfileError::NoProfile),
            )) => Self::NoGameProfile,
        }
    }
}
