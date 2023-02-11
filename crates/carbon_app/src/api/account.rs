use chrono::{DateTime, Utc};
use rspc::{RouterBuilderLike, Type};
use serde::Serialize;

use crate::api::keys::account::*;
use crate::api::router::router;
use crate::managers::{account, Managers};
use carbon_domain::account as domain;

pub(super) fn mount() -> impl RouterBuilderLike<Managers> {
    router! {
        query GET_ACTIVE_UUID[app, _: ()] {
            Ok(app.account_manager.get_active_uuid().await?)
        }

        mutation SET_ACTIVE_UUID[app, uuid: Option<String>] {
            app.account_manager.set_active_uuid(uuid).await?;
            Ok(())
        }

        query GET_ACCOUNTS[app, _: ()] {
            Ok(app.account_manager
               .get_account_list()
               .await?
               .into_iter()
               .map(AccountEntry::from)
               .collect::<Vec<_>>())
        }

        query GET_ACCOUNT_STATUS[app, uuid: String] {
            Ok(app.account_manager.get_account_status(uuid).await?
               .map(AccountStatus::from))
        }

        mutation DELETE_ACCOUNT[app, uuid: String] {
            app.account_manager.delete_account(uuid).await?;
            Ok(())
        }

        mutation ENROLL_BEGIN[app, _: ()] {
            app.account_manager.begin_enrollment().await?;
            Ok(())
        }

        mutation ENROLL_CANCEL[app, _: ()] {
            app.account_manager.cancel_enrollment().await?;
            Ok(())
        }

        query ENROLL_GET_STATUS[app, _: ()] {
            Ok(EnrollmentStatus::from(
                app.account_manager.get_enrollment_status().await?
            ))
        }

        query ENROLL_FINALIZE[app, _: ()] {
            app.account_manager.finalize_enrollment().await?;
            Ok(())
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
    Failed(String),
}

#[derive(Type, Serialize)]
struct DeviceCode {
    user_code: String,
    verification_uri: String,
    expires_at: DateTime<Utc>,
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
            Status::Failed(msg) => Self::Failed(msg),
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
