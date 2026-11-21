//! Why a change to a world, or to the data packs in it, was refused. The
//! desktop words each refusal from its code and data, in the player's
//! language; the sentence here is what logs show, and what a desktop that
//! does not know the code falls back to.
use crate::error::FeErrorCode;
use serde_json::json;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WorldRefusal {
    /// The instance's game is queued, preparing or running: it has the
    /// worlds open and writes them when it closes.
    #[error(
        "This instance is running or being installed. Close the game or wait for it to finish, then try again."
    )]
    GameActive,
    /// The save the user selected is no longer in the instance.
    #[error("Selected world no longer exists")]
    WorldGone,
    /// The folder of that name is no save the launcher can change.
    #[error("{0}")]
    NotASave(SaveFault),
    /// The save of that name is not the one the world list showed.
    #[error("Selected world changed. Refresh the world list and select it again")]
    WorldChanged,
    /// The save was swapped while the data pack was on its way.
    #[error("Selected world changed during copying; no datapack published")]
    WorldChangedDuringCopy,
    /// The world holds another entry under the data pack's file name.
    #[error("This world already contains that datapack filename; existing content was retained")]
    NameTaken { filename: String },
    /// The row asked for is not one of the instance's world data packs
    /// (any more).
    #[error("This data pack is no longer in the world. Refresh the world list and try again")]
    PackGone,
    /// What is on disk is not the entry the world list showed.
    #[error("This data pack changed since the world list was read. Refresh it and try again")]
    PackChanged,
    #[error("A data pack can only be replaced by one installed into its own world")]
    OtherWorld,
    #[error("A data pack folder cannot be replaced; remove it instead")]
    FolderNotReplaced,
    /// A version the user picked that the game would not load from a world:
    /// it is not tagged as a data pack, or its file is no zip.
    #[error("This version is not a data pack")]
    NotADatapackVersion,
    /// The instance's copy to put into a world is no longer in the
    /// instance's data pack folder.
    #[error("Staged datapack is missing")]
    InstancePackGone,
    /// The instance's copy was rewritten while it was copied into a world.
    #[error("Staged datapack changed during copying; retry with the current artifact")]
    InstancePackChanged,
    /// Only a zip is copied from the instance into a world.
    #[error("Select a ZIP datapack for this world")]
    NotAZip,
    /// The zip is no data pack the game would load.
    #[error("{0}")]
    InvalidPack(PackFault),
    /// CurseForge gives no download address for the file: its author does
    /// not allow other launchers to download it.
    #[error("mod cannot be downloaded without privileged api key")]
    NotDownloadable,
    #[error("Failed to download data pack {filename}: {cause}")]
    DownloadFailed { filename: String, cause: String },
    /// No newer build may replace the pack, as for any addon.
    #[error("unable to find newer mod version in available update channels")]
    NoUpdate,
}

/// What makes a folder in `saves` no save the launcher can change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SaveFault {
    #[error("Selected world is not an owned save directory")]
    NotAFolder,
    #[error("Selected world installation is incomplete")]
    InstallIncomplete,
    #[error("Selected directory is not an existing Minecraft save")]
    NoLevelDat,
    #[error("World metadata is not a regular file")]
    LevelDatNotAFile,
}

/// What makes a zip no data pack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PackFault {
    #[error("Datapack ZIP must contain pack.mcmeta at its root")]
    NoPackMcmeta,
    #[error("Datapack metadata is too large")]
    MetadataTooLarge,
    #[error("Datapack pack.mcmeta is not valid JSON")]
    MetadataNotJson,
    #[error("Invalid datapack metadata")]
    NoPackSection,
}

impl FeErrorCode for WorldRefusal {
    fn error_code(&self) -> &'static str {
        match self {
            Self::GameActive => "INSTANCE_GAME_ACTIVE",
            Self::WorldGone => "WORLD_GONE",
            Self::NotASave(_) => "WORLD_NOT_A_SAVE",
            Self::WorldChanged => "WORLD_CHANGED",
            Self::WorldChangedDuringCopy => "WORLD_CHANGED_DURING_COPY",
            Self::NameTaken { .. } => "DATAPACK_NAME_TAKEN",
            Self::PackGone => "DATAPACK_GONE",
            Self::PackChanged => "DATAPACK_CHANGED",
            Self::OtherWorld => "DATAPACK_IN_OTHER_WORLD",
            Self::FolderNotReplaced => "DATAPACK_FOLDER_NOT_REPLACED",
            Self::NotADatapackVersion => "NOT_A_DATAPACK_VERSION",
            Self::InstancePackGone => "INSTANCE_DATAPACK_GONE",
            Self::InstancePackChanged => "INSTANCE_DATAPACK_CHANGED",
            Self::NotAZip => "DATAPACK_NOT_A_ZIP",
            Self::InvalidPack(_) => "DATAPACK_INVALID",
            Self::NotDownloadable => "DATAPACK_NOT_DOWNLOADABLE",
            Self::DownloadFailed { .. } => "DATAPACK_DOWNLOAD_FAILED",
            Self::NoUpdate => "NO_DATAPACK_UPDATE",
        }
    }

    fn error_data(&self) -> Option<serde_json::Value> {
        match self {
            Self::NotASave(fault) => Some(json!({
                "reason": match fault {
                    SaveFault::NotAFolder => "NOT_A_FOLDER",
                    SaveFault::InstallIncomplete => "INSTALL_INCOMPLETE",
                    SaveFault::NoLevelDat => "NO_LEVEL_DAT",
                    SaveFault::LevelDatNotAFile => "LEVEL_DAT_NOT_A_FILE",
                }
            })),
            Self::NameTaken { filename } => Some(json!({ "filename": filename })),
            Self::InvalidPack(fault) => Some(json!({
                "reason": match fault {
                    PackFault::NoPackMcmeta => "NO_PACK_MCMETA",
                    PackFault::MetadataTooLarge => "METADATA_TOO_LARGE",
                    PackFault::MetadataNotJson => "METADATA_NOT_JSON",
                    PackFault::NoPackSection => "NO_PACK_SECTION",
                }
            })),
            Self::DownloadFailed { filename, cause } => {
                Some(json!({ "filename": filename, "cause": cause }))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{FeError, coded_cause, flattened};

    fn every_refusal() -> Vec<WorldRefusal> {
        let mut all = vec![
            WorldRefusal::GameActive,
            WorldRefusal::WorldGone,
            WorldRefusal::WorldChanged,
            WorldRefusal::WorldChangedDuringCopy,
            WorldRefusal::NameTaken {
                filename: "pack.zip".into(),
            },
            WorldRefusal::PackGone,
            WorldRefusal::PackChanged,
            WorldRefusal::OtherWorld,
            WorldRefusal::FolderNotReplaced,
            WorldRefusal::NotADatapackVersion,
            WorldRefusal::InstancePackGone,
            WorldRefusal::InstancePackChanged,
            WorldRefusal::NotAZip,
            WorldRefusal::NotDownloadable,
            WorldRefusal::DownloadFailed {
                filename: "pack.zip".into(),
                cause: "connection reset".into(),
            },
            WorldRefusal::NoUpdate,
        ];
        all.extend(
            [
                SaveFault::NotAFolder,
                SaveFault::InstallIncomplete,
                SaveFault::NoLevelDat,
                SaveFault::LevelDatNotAFile,
            ]
            .map(WorldRefusal::NotASave),
        );
        all.extend(
            [
                PackFault::NoPackMcmeta,
                PackFault::MetadataTooLarge,
                PackFault::MetadataNotJson,
                PackFault::NoPackSection,
            ]
            .map(WorldRefusal::InvalidPack),
        );
        all
    }

    /// What the desktop is sent for `error`: its causes, outermost first.
    fn sent(error: &anyhow::Error) -> serde_json::Value {
        serde_json::to_value(FeError::from_anyhow(error)).unwrap()["cause"].take()
    }

    #[test]
    fn a_refused_mutation_carries_the_code_the_data_and_the_sentence() {
        let mut seen = std::collections::HashSet::new();
        for refusal in every_refusal() {
            let causes = sent(&anyhow::Error::new(refusal.clone()));
            let cause = &causes[0];
            assert_eq!(cause["code"], refusal.error_code(), "{refusal:?}");
            assert_eq!(cause["display"], refusal.to_string(), "{refusal:?}");
            assert_eq!(
                cause.get("data").cloned(),
                refusal.error_data(),
                "{refusal:?}"
            );
            // One code per refusal; the faults of one refusal tell
            // themselves apart by their reason.
            seen.insert((
                refusal.error_code(),
                refusal.error_data().map(|data| data["reason"].to_string()),
            ));
        }
        assert_eq!(seen.len(), every_refusal().len());
    }

    #[test]
    fn a_refusal_under_another_errors_context_is_still_found() {
        let error = anyhow::Error::new(WorldRefusal::PackChanged).context("while updating");
        let causes = sent(&error);
        assert_eq!(causes[0]["display"], "while updating");
        assert!(causes[0].get("code").is_none());
        assert_eq!(causes[1]["code"], "DATAPACK_CHANGED");
        assert_eq!(
            coded_cause(&error),
            Some(("DATAPACK_CHANGED".to_owned(), None))
        );
    }

    /// A failed task keeps its error as one line; the line still carries the
    /// code and data of the refusal in it.
    #[test]
    fn a_failed_task_keeps_the_code_of_the_refusal_it_shows_as_one_line() {
        let refusal = WorldRefusal::NameTaken {
            filename: "pack.zip".into(),
        };
        let error = anyhow::Error::new(refusal.clone()).context("Could not publish");
        let causes = sent(&flattened(&error));
        assert_eq!(causes.as_array().unwrap().len(), 1, "{causes}");
        assert_eq!(
            causes[0]["display"],
            format!("Could not publish: {refusal}")
        );
        assert_eq!(causes[0]["code"], "DATAPACK_NAME_TAKEN");
        assert_eq!(causes[0]["data"], json!({ "filename": "pack.zip" }));

        // An error without a code is the line alone, as before.
        let plain = flattened(&anyhow::anyhow!("disk full").context("Could not publish"));
        let causes = sent(&plain);
        assert_eq!(causes[0]["display"], "Could not publish: disk full");
        assert!(causes[0].get("code").is_none());
    }
}
