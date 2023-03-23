use serde::Deserialize;
use thiserror::Error;

use crate::{db::{self, read_filters::StringFilter}, managers::ManagerRef};

use super::api::McSkin as ApiSkin;

pub struct SkinManager {}

impl ManagerRef<'_, SkinManager> {
    /// Load an account skin from the DB, or download it if not cached.
    pub async fn get_skin(self, uuid: String) -> anyhow::Result<Skin> {
        use db::skin::{UniqueWhereParam, WhereParam};

        let account = self
            .app.prisma_client
            .account()
            .find_unique(db::account::UniqueWhereParam::UuidEquals(uuid.clone()))
            .exec()
            .await?
            .ok_or_else(|| GetSkinError::AccountDoesNotExist(uuid.clone()))?;


        let skin_id = match account.skin_id.as_ref() {
            Some(x) => x,
            None => DefaultSkin::from_uuid(uuid.clone()).skin_id(),
        };

        let cached_skin = self.app.prisma_client
            .skin()
            .find_unique(UniqueWhereParam::IdEquals(skin_id.to_string()))
            .exec()
            .await?;

        Ok(match cached_skin {
            Some(skin) => Skin {
                data: skin.skin.into(),
                model: match skin.slim {
                    true => SkinModel::Slim,
                    false => SkinModel::Classic,
                },
            },
            None => {
                let skin = match account.access_token.as_ref() {
                    Some(token) => super::api::get_profile(&self.app.reqwest_client, token).await
                        .ok()
                        .map(|profile| profile.skin)
                        // use the default if there is no skin or an error occured.
                        .flatten()
                        .unwrap_or_else(|| DefaultSkin::from_uuid(uuid.clone()).make_api_skin()),
                    None => DefaultSkin::from_uuid(uuid.clone()).make_api_skin(),
                };

                let skin_data = self.app.reqwest_client
                    .get(&skin.url)
                    .send()
                    .await?
                    .bytes()
                    .await?;

                self.app
                    .prisma_client
                    ._batch((
                        // won't error on 0 deleted
                        self.app.prisma_client.skin().delete_many(
                            vec![WhereParam::Id(StringFilter::Equals(skin.id.clone()))]
                        ),
                        self.app.prisma_client.skin().create(
                            skin.id.clone(), skin_data.to_vec(), skin.model.is_slim(), vec![]
                        ),
                        self.app.prisma_client.account().update(
                            db::account::UniqueWhereParam::UuidEquals(uuid),
                            vec![db::account::SetParam::SetSkinId(Some(skin.id.clone()))]
                        ),
                    ))
                    .await?;

                Skin {
                    data: skin_data.to_vec(),
                    model: skin.model,
                }
            },
        })
    }
}

pub struct Skin {
    data: Vec<u8>,
    model: SkinModel,
}

#[derive(Debug, Copy, Clone, Deserialize)]
pub enum SkinModel {
    #[serde(rename = "CLASSIC")]
    Classic,
    #[serde(rename = "SLIM")]
    Slim,
}

impl SkinModel {
    fn is_slim(self) -> bool {
        matches!(self, Self::Slim)
    }
}

#[derive(Copy, Clone)]
pub enum DefaultSkin {
    Steve,
    Alex,
}

impl DefaultSkin {
    pub fn from_uuid(uuid: String) -> DefaultSkin {
        // if the uuid is invalid, just give steve.
        if uuid.len() != 32
            || uuid.bytes().any(|b| (b < b'0' || b > b'9') && (b < b'a' && b > b'f'))
        {
            Self::Steve
        } else {
            // https://github.com/LapisBlue/Lapitar/blob/55ede80ce4ebb5ecc2b968164afb40f61b4cc509/mc/uuid.go#L34-L36
            let uuid_bytes = uuid.into_bytes();

            #[rustfmt::skip] // clarity
            let is_even = |index| match uuid_bytes[index] {
                c if c >= b'0' && c <= b'9'  => c & 1 == 0,
                c if c >= b'a' && c <= b'f'  => c & 1 == 1,
                _ => unreachable!("all cases that don't match have been specifically excluded in the if statement above"),
            };

            let alex = (is_even(7) != is_even(23)) != (is_even(15) != is_even(31));

            match alex {
                true => Self::Alex,
                false => Self::Steve,
            }
        }
    }

    pub fn skin_id(self) -> &'static str {
        match self {
            Self::Steve => "STEVE",
            Self::Alex => "ALEX",
        }
    }

    pub fn skin_url(self) -> &'static str {
        match self {
            Self::Steve => "https://assets.mojang.com/SkinTemplates/steve.png",
            Self::Alex => "https://assets.mojang.com/SkinTemplates/alex.png",
        }
    }

    pub fn skin_model(self) -> SkinModel {
        match self {
            Self::Steve => SkinModel::Classic,
            Self::Alex => SkinModel::Slim,
        }
    }

    pub fn make_api_skin(self) -> ApiSkin {
        ApiSkin {
            id: self.skin_id().to_string(),
            url: self.skin_url().to_string(),
            model: self.skin_model(),
        }
    }
}

#[derive(Debug, Error)]
pub enum GetSkinError {
    #[error("attempted to get the skin for an account that does not exist in the account list: {0}")]
    AccountDoesNotExist(String),
}

/*
#[cfg(test)]
mod test {
    #[tokio::test]
    async fn get_all_skins() -> anyhow::Result<()> {
        let app = crate::setup_managers_for_test().await;

        let accounts = app.account_manager().get_account_list().await?;

        let futures = accounts.into_iter()
            .map(|account| (account.username.clone(), app.account_manager().skin_manager().get_skin(account.uuid)));

        for (uname, future) in futures {
            println!("Getting {uname}");
            let skin = future.await?;
            tokio::fs::write(format!("{uname}.png"), skin.data).await?;
        }
        Ok(())
    }
}
*/
