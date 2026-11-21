//! The theme preferences: the Customize switches and the tile-size flag.

use carbon_repos::dbtypes::DbDateTime;
use carbon_repos::repos::frontend_preference;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::ThemeManager;
use crate::api::keys::themes::GET_PREFERENCES;
use crate::api::themes::FEThemePreferencesPatch;
use crate::managers::ManagerRef;

pub const PREFERENCES_KEY: &str = "theme_preferences";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ThemePreferences {
    pub sounds: bool,
    pub effects: bool,
    pub tile_size_chosen: bool,
}

impl Default for ThemePreferences {
    fn default() -> Self {
        Self {
            sounds: false,
            effects: true,
            tile_size_chosen: false,
        }
    }
}

impl ThemePreferences {
    fn merged(mut self, patch: &FEThemePreferencesPatch) -> Self {
        if let Some(v) = patch.sounds {
            self.sounds = v;
        }
        if let Some(v) = patch.effects {
            self.effects = v;
        }
        if let Some(v) = patch.tile_size_chosen {
            self.tile_size_chosen = v;
        }
        self
    }
}

impl ManagerRef<'_, ThemeManager> {
    /// A failed read is an error, so no caller saves a default over the real
    /// row. Only a row that isn't valid JSON falls back to the default.
    pub async fn preferences(self) -> anyhow::Result<ThemePreferences> {
        super::load_json_pref(&self.app.db, PREFERENCES_KEY, "theme preferences").await
    }

    pub async fn set_preferences(
        self,
        patch: FEThemePreferencesPatch,
    ) -> anyhow::Result<ThemePreferences> {
        let _prefs = self.prefs.clone().lock_owned().await;
        let next = self.preferences().await?.merged(&patch);
        frontend_preference::upsert_preference(
            &self.app.db,
            PREFERENCES_KEY,
            &serde_json::to_string(&next)?,
            DbDateTime(Utc::now().fixed_offset()),
        )
        .await?;
        self.app.invalidate(GET_PREFERENCES, None);
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::themes::FEThemePreferencesPatch;
    use carbon_repos::dbtypes::DbDateTime;
    use carbon_repos::repos::frontend_preference;

    #[tokio::test]
    async fn setting_preferences_invalidates_them() {
        use crate::api::keys::themes::GET_PREFERENCES;
        let mut app = crate::setup_managers_for_test().await;
        while app.invalidation_recv.try_recv().is_ok() {}
        app.theme_manager()
            .set_preferences(FEThemePreferencesPatch {
                effects: Some(false),
                ..Default::default()
            })
            .await
            .unwrap();
        let mut keys = Vec::new();
        while let Ok(event) = app.invalidation_recv.try_recv() {
            keys.push(event.key);
        }
        assert!(keys.contains(&GET_PREFERENCES.full), "{keys:?}");
    }

    #[tokio::test]
    async fn preferences_default_and_merge() {
        let app = crate::setup_managers_for_test().await;
        let themes = app.theme_manager();
        let d = themes.preferences().await.unwrap();
        assert_eq!(
            d,
            ThemePreferences {
                sounds: false,
                effects: true,
                tile_size_chosen: false
            }
        );
        let p = themes
            .set_preferences(FEThemePreferencesPatch {
                sounds: Some(true),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(
            (p.sounds, p.effects, p.tile_size_chosen),
            (true, true, false)
        );
        let p = themes
            .set_preferences(FEThemePreferencesPatch {
                effects: Some(false),
                tile_size_chosen: Some(true),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(
            (p.sounds, p.effects, p.tile_size_chosen),
            (true, false, true)
        );
        assert!(themes.preferences().await.unwrap().tile_size_chosen);

        frontend_preference::upsert_preference(
            &app.db,
            PREFERENCES_KEY,
            "nope",
            DbDateTime(chrono::Utc::now().fixed_offset()),
        )
        .await
        .unwrap();
        assert_eq!(
            themes.preferences().await.unwrap(),
            ThemePreferences::default()
        );
    }
}
