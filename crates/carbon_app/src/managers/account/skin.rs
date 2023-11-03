use std::io::Cursor;

use anyhow::ensure;
use image::{GenericImageView, ImageOutputFormat};
use thiserror::Error;

use crate::{
    db::{self, read_filters::StringFilter},
    managers::ManagerRef,
};

use super::api::McSkin as ApiSkin;

pub struct SkinManager {}

impl ManagerRef<'_, SkinManager> {
    /// Load an account skin from the DB, or download it if not cached.
    pub async fn get_skin(self, uuid: String) -> anyhow::Result<Skin> {
        use db::skin::{UniqueWhereParam, WhereParam};

        let account = self
            .app
            .prisma_client
            .account()
            .find_unique(db::account::UniqueWhereParam::UuidEquals(
                uuid.clone(),
            ))
            .exec()
            .await?
            .ok_or_else(|| GetSkinError::AccountDoesNotExist(uuid.clone()))?;

        let skin_id = match account.skin_id.as_ref() {
            Some(x) => x,
            None => DefaultSkin::from_uuid(uuid.clone()).skin_id(),
        };

        let cached_skin = self
            .app
            .prisma_client
            .skin()
            .find_unique(UniqueWhereParam::IdEquals(skin_id.to_string()))
            .exec()
            .await?;

        Ok(match cached_skin {
            Some(skin) => Skin {
                data: skin.skin.into(),
            },
            None => {
                let skin = match account.access_token.as_ref() {
                    Some(token) => {
                        super::api::get_profile(&self.app.reqwest_client, token)
                            .await
                            .ok()
                            .map(Result::ok)
                            .flatten()
                            .map(|profile| profile.skin)
                            // use the default if there is no skin or an error occured.
                            .flatten()
                            .unwrap_or_else(|| {
                                DefaultSkin::from_uuid(uuid.clone())
                                    .make_api_skin()
                            })
                    }
                    None => {
                        DefaultSkin::from_uuid(uuid.clone()).make_api_skin()
                    }
                };

                let skin_data = self
                    .app
                    .reqwest_client
                    .get(&skin.url)
                    .send()
                    .await?
                    .bytes()
                    .await?;

                self.app
                    .prisma_client
                    ._batch((
                        // won't error on 0 deleted
                        self.app.prisma_client.skin().delete_many(vec![
                            WhereParam::Id(StringFilter::Equals(
                                skin.id.clone(),
                            )),
                        ]),
                        self.app.prisma_client.skin().create(
                            skin.id.clone(),
                            skin_data.to_vec(),
                            vec![],
                        ),
                        self.app.prisma_client.account().update(
                            db::account::UniqueWhereParam::UuidEquals(uuid),
                            vec![db::account::SetParam::SetSkinId(Some(
                                skin.id.clone(),
                            ))],
                        ),
                    ))
                    .await?;

                Skin {
                    data: skin_data.to_vec(),
                }
            }
        })
    }

    pub async fn make_head(self, uuid: String) -> anyhow::Result<Vec<u8>> {
        let skin = self.get_skin(uuid).await?.data;
        let head =
            tokio::task::spawn_blocking(move || stitch_head(&skin)).await??;
        Ok(head)
    }
}

fn stitch_head(image: &[u8]) -> anyhow::Result<Vec<u8>> {
    use image::imageops::*;

    let reader = image::io::Reader::new(Cursor::new(image))
        .with_guessed_format()
        .expect("cursor io cannot fail");

    let image = reader.decode()?;

    ensure!(
        image.width() == 64 && (image.height() == 64 || image.height() == 32),
        "cannot stitch head from unsupported skin image"
    );

    let head = image.view(8, 8, 8, 8);
    let hat_front = image.view(40, 8, 8, 8);
    let hat_back = image.view(56, 8, 8, 8);

    let mut target = image::RgbaImage::new(144, 144);

    overlay(
        &mut target,
        &resize(&hat_back.to_image(), 136, 136, FilterType::Nearest),
        4,
        4,
    );
    overlay(
        &mut target,
        &resize(&head.to_image(), 128, 128, FilterType::Nearest),
        8,
        8,
    );
    overlay(
        &mut target,
        &resize(&hat_front.to_image(), 144, 144, FilterType::Nearest),
        0,
        0,
    );

    let mut output = Vec::<u8>::new();
    target.write_to(&mut Cursor::new(&mut output), ImageOutputFormat::Png)?;
    Ok(output)
}

pub struct Skin {
    data: Vec<u8>,
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
            || uuid
                .bytes()
                .any(|b| !b.is_ascii_digit() && (b < b'a' && b > b'f'))
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

            let alex =
                (is_even(7) != is_even(23)) != (is_even(15) != is_even(31));

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

    pub fn make_api_skin(self) -> ApiSkin {
        ApiSkin {
            id: self.skin_id().to_string(),
            url: self.skin_url().to_string(),
        }
    }
}

#[derive(Debug, Error)]
pub enum GetSkinError {
    #[error(
        "attempted to get the skin for an account that does not exist in the account list: {0}"
    )]
    AccountDoesNotExist(String),
}

/*
#[cfg(test)]
mod test {
    use crate::managers::account::skin::stitch_head;

    #[tokio::test]
    async fn get_all_skins() -> anyhow::Result<()> {
        let app = crate::setup_managers_for_test().await;

        let accounts = app.account_manager().get_account_list().await?;

        let futures = accounts.into_iter().map(|account| {
            (
                account.username.clone(),
                app.account_manager().skin_manager().get_skin(account.uuid),
            )
        });

        for (uname, future) in futures {
            println!("Getting {uname}");
            let skin = future.await?;
            tokio::fs::write(format!("{uname}.png"), &skin.data).await?;
            println!("Stitching head");
            let head = stitch_head(&skin.data)?;
            tokio::fs::write(format!("{uname}_head.png"), &head).await?;
        }
        Ok(())
    }
}
*/
