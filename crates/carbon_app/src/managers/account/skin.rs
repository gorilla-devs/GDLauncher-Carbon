use super::api::McSkin as ApiSkin;
use crate::managers::ManagerRef;
use anyhow::ensure;
use image::{GenericImageView, ImageFormat};
use std::io::Cursor;
use thiserror::Error;

pub struct SkinManager {}

#[cfg(test)]
mod task20_tests;

impl ManagerRef<'_, SkinManager> {
    /// Load an account skin from the DB, or download it if not cached.
    pub async fn get_skin(self, uuid: String) -> anyhow::Result<Skin> {
        get_skin(&self.app.db, &self.app.reqwest_client, uuid).await
    }

    pub async fn make_head(self, uuid: String) -> anyhow::Result<Vec<u8>> {
        let skin = self.get_skin(uuid).await?.data;
        let head = carbon_scheduler::cpu_block(|| stitch_head(&skin)).await?;
        Ok(head)
    }
}

async fn get_skin(
    db: &carbon_repos::db_exec::Db,
    client: &reqwest_middleware::ClientWithMiddleware,
    uuid: String,
) -> anyhow::Result<Skin> {
    use carbon_repos::repos::{account as account_repo, skin as skin_repo};
    let account = account_repo::get_account(db, &uuid)
        .await?
        .ok_or_else(|| GetSkinError::AccountDoesNotExist(uuid.clone()))?;
    let default = DefaultSkin::from_uuid(uuid.clone());
    let skin_id = account.skin_id.as_deref().unwrap_or(default.skin_id());
    if let Some(data) = validated_cache(db, skin_id).await? {
        return Ok(Skin { data });
    }

    let preserve_custom = account
        .skin_id
        .as_deref()
        .is_some_and(|id| id != "STEVE" && id != "ALEX");
    let selected = match account.access_token.as_ref() {
        Some(token) => super::api::get_profile(client, token)
            .await
            .ok()
            .and_then(Result::ok)
            .and_then(|profile| profile.skin),
        None => None,
    }
    .unwrap_or_else(|| default.make_api_skin());

    let fetched = match validated_cache(db, &selected.id).await? {
        Some(data) => Ok(data),
        None => download_skin(client, &selected).await,
    };
    let data = match fetched {
        Ok(data) => data,
        // Recovery is display-only. Never relink a custom account to a default
        // just because its origin is temporarily unavailable.
        Err(error) if preserve_custom => {
            let data = match validated_cache(db, default.skin_id()).await? {
                Some(data) => data,
                None => match download_skin(client, &default.make_api_skin()).await {
                    Ok(data) => data,
                    Err(_) => return Err(error),
                },
            };
            skin_repo::cache_skin(db, default.skin_id(), &data).await?;
            return Ok(Skin { data });
        }
        Err(error) => return Err(error),
    };
    if preserve_custom && (selected.id == "STEVE" || selected.id == "ALEX") {
        skin_repo::cache_skin(db, &selected.id, &data).await?;
    } else {
        skin_repo::replace_skin_and_link_account(db, selected.id, data.clone(), uuid).await?;
    }
    Ok(Skin { data })
}

async fn validated_cache(
    db: &carbon_repos::db_exec::Db,
    id: &str,
) -> anyhow::Result<Option<Vec<u8>>> {
    use carbon_repos::repos::skin;
    let Some(row) = skin::get_skin(db, id).await? else {
        return Ok(None);
    };
    if validate_skin(&row.skin).await.is_ok() {
        return Ok(Some(row.skin));
    }
    if skin::evict_skin_if_unchanged(db, id, &row.skin).await? == 0 {
        // Another writer repaired it after our read. Recheck once, never evict
        // bytes we have not observed and never recurse through damaged defaults.
        if let Some(current) = skin::get_skin(db, id).await? {
            if validate_skin(&current.skin).await.is_ok() {
                return Ok(Some(current.skin));
            }
        }
    }
    Ok(None)
}

async fn download_skin(
    client: &reqwest_middleware::ClientWithMiddleware,
    skin: &ApiSkin,
) -> anyhow::Result<Vec<u8>> {
    let response = client
        .get(&skin.url)
        .header("avoid-caching", "true")
        .send()
        .await?
        .error_for_status()?;
    ensure!(
        response.status().is_success(),
        "skin request returned {}",
        response.status()
    );
    let data = response.bytes().await?.to_vec();
    validate_skin(&data).await?;
    Ok(data)
}

async fn validate_skin(data: &[u8]) -> anyhow::Result<()> {
    carbon_scheduler::cpu_block(|| decode_skin(data).map(|_| ())).await
}

fn decode_skin(data: &[u8]) -> anyhow::Result<image::DynamicImage> {
    let image = image::io::Reader::new(Cursor::new(data))
        .with_guessed_format()?
        .decode()?;
    ensure!(
        image.width() == 64 && (image.height() == 64 || image.height() == 32),
        "cannot stitch head from unsupported skin image"
    );
    Ok(image)
}

fn stitch_head(image: &[u8]) -> anyhow::Result<Vec<u8>> {
    use image::imageops::*;

    let image = decode_skin(image)?;

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
    target.write_to(&mut Cursor::new(&mut output), ImageFormat::Png)?;
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
                .any(|b| !b.is_ascii_digit() && (b < b'a' || b > b'f'))
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
