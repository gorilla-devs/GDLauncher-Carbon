//! Season rules for built-in themes, as pure functions of today's date.
//!
//! A season is a window of calendar days that wraps the new year when its end
//! is before its start. Its id is the year the window started in, so Dec 2026
//! to Jan 2027 is season "2026". The rules (`decide` and friends) are pure and
//! the registry is embedded with `include_str!`; neither reads the clock or
//! the database. `season_today`, the stored state and the manager methods do.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::OnceLock;

use carbon_repos::dbtypes::DbDateTime;
use carbon_repos::repos::app_configuration::AppConfigurationPatch;
use carbon_repos::repos::frontend_preference;
use chrono::{Datelike, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::error::ThemeError;
use super::extras::{as_object, warn_unknown};
use super::files::{FileKind, ThemeFiles};
use super::manifest::check_path;
use super::{ThemeManager, theme_dir};
use crate::api::keys::settings::GET_SETTINGS;
use crate::api::keys::themes::GET_SEASON;
use crate::api::themes::{FESeasonNotice, FESeasonStatus, FESeasonalTheme};
use crate::managers::ManagerRef;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MonthDay {
    pub month: u32,
    pub day: u32,
}

/// Inclusive. Wraps the year when `end` is before `start`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SeasonWindow {
    pub start: MonthDay,
    pub end: MonthDay,
}

impl MonthDay {
    fn key(self) -> (u32, u32) {
        (self.month, self.day)
    }
}

impl SeasonWindow {
    fn wraps(&self) -> bool {
        self.end.key() < self.start.key()
    }

    /// Days from `available.start`, along a non-leap year.
    fn offset(&self, md: MonthDay) -> i64 {
        let day_of_year = |m: MonthDay| {
            NaiveDate::from_ymd_opt(2001, m.month, m.day).map_or(0, |d| i64::from(d.ordinal0()))
        };
        (day_of_year(md) - day_of_year(self.start)).rem_euclid(365)
    }

    /// True when `inner` lies inside this window without wrapping past its end.
    fn encloses(&self, inner: &SeasonWindow) -> bool {
        let (a, b) = (self.offset(inner.start), self.offset(inner.end));
        a <= b && b <= self.offset(self.end)
    }

    fn contains_md(&self, md: MonthDay) -> bool {
        let k = md.key();
        if self.wraps() {
            k >= self.start.key() || k <= self.end.key()
        } else {
            k >= self.start.key() && k <= self.end.key()
        }
    }

    pub fn contains(&self, d: NaiveDate) -> bool {
        self.contains_md(MonthDay {
            month: d.month(),
            day: d.day(),
        })
    }

    /// The year the window holding `d` started in.
    pub fn season_id(&self, d: NaiveDate) -> Option<String> {
        if !self.contains(d) {
            return None;
        }
        let in_new_year_part = self.wraps() && (d.month(), d.day()) <= self.end.key();
        let year = if in_new_year_part {
            d.year() - 1
        } else {
            d.year()
        };
        Some(year.to_string())
    }

    /// The last day of the window holding `d`.
    pub fn end_date(&self, d: NaiveDate) -> Option<NaiveDate> {
        if !self.contains(d) {
            return None;
        }
        let in_old_year_part = self.wraps() && (d.month(), d.day()) >= self.start.key();
        let year = if in_old_year_part {
            d.year() + 1
        } else {
            d.year()
        };
        NaiveDate::from_ymd_opt(year, self.end.month, self.end.day)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ManifestSeason {
    pub available: SeasonWindow,
    pub auto_select: Option<SeasonWindow>,
    pub art: Option<String>,
}

fn invalid<T>(msg: impl Into<String>) -> Result<T, ThemeError> {
    Err(ThemeError::Invalid(msg.into()))
}

fn parse_month_day(raw: &str, field: &str) -> Result<MonthDay, ThemeError> {
    let b = raw.as_bytes();
    let well_formed = b.len() == 5
        && b[2] == b'-'
        && b[..2].iter().all(u8::is_ascii_digit)
        && b[3..].iter().all(u8::is_ascii_digit);
    if well_formed {
        let month: u32 = raw[..2].parse().unwrap_or(0);
        let day: u32 = raw[3..].parse().unwrap_or(0);
        // 2001 isn't a leap year, so 02-29 is refused.
        if NaiveDate::from_ymd_opt(2001, month, day).is_some() {
            return Ok(MonthDay { month, day });
        }
    }
    invalid(format!(
        "{field} must be \"MM-DD\" with a real day, like \"12-01\""
    ))
}

fn parse_window(v: &Value, field: &str) -> Result<SeasonWindow, ThemeError> {
    let pair = v.as_array().filter(|a| a.len() == 2);
    let Some([start, end]) = pair.map(Vec::as_slice) else {
        return invalid(format!(
            "{field} must be two dates, like [\"12-01\", \"01-31\"]"
        ));
    };
    let (Some(start), Some(end)) = (start.as_str(), end.as_str()) else {
        return invalid(format!(
            "{field} must be two dates, like [\"12-01\", \"01-31\"]"
        ));
    };
    Ok(SeasonWindow {
        start: parse_month_day(start, field)?,
        end: parse_month_day(end, field)?,
    })
}

/// `files` of `None` skips the check that `art` is in the theme.
pub fn parse_season(
    v: &Value,
    files: Option<&ThemeFiles>,
    warnings: &mut Vec<String>,
) -> Result<ManifestSeason, ThemeError> {
    let o = as_object(v, "season")?;
    warn_unknown(
        o,
        &["available", "autoSelect", "art"],
        "season",
        "season",
        warnings,
    );
    let Some(available) = o.get("available") else {
        return invalid("season.available is required");
    };
    let available = parse_window(available, "season.available")?;
    let auto_select = match o.get("autoSelect") {
        None | Some(Value::Null) => None,
        Some(v) => {
            let w = parse_window(v, "season.autoSelect")?;
            if !available.encloses(&w) {
                return invalid("season.autoSelect must lie inside season.available");
            }
            Some(w)
        }
    };
    let art = match o.get("art") {
        None | Some(Value::Null) => None,
        Some(Value::String(raw)) => match files {
            Some(files) => Some(check_path(
                raw,
                "season.art",
                FileKind::Image,
                "an image",
                files,
            )?),
            None => Some(raw.clone()),
        },
        Some(_) => return invalid("season.art must be a path"),
    };
    Ok(ManifestSeason {
        available,
        auto_select,
        art,
    })
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SeasonState {
    /// The theme to go back to when the season ends or the user switches back.
    #[serde(default)]
    pub previous_theme: Option<String>,
    /// Theme id to the season id it was last auto-selected in.
    #[serde(default)]
    pub applied_for: BTreeMap<String, String>,
    /// Theme ids the user pressed "Switch back" on: never auto-selected again.
    #[serde(default)]
    pub opted_out: Vec<String>,
    #[serde(default)]
    pub notice: Option<StoredNotice>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum StoredNotice {
    #[serde(rename_all = "camelCase")]
    Selected { theme_id: String },
    #[serde(rename_all = "camelCase")]
    Ended { theme_id: String },
}

pub struct Seasonal<'a> {
    pub id: &'a str,
    pub season: &'a ManifestSeason,
}

/// `theme` is `Some` only when the active theme must change.
pub struct Decision {
    pub theme: Option<String>,
    pub state: SeasonState,
}

fn drop_stale_selected_notice(state: &mut SeasonState, current: &str) {
    if matches!(&state.notice, Some(StoredNotice::Selected { theme_id }) if theme_id != current) {
        state.notice = None;
    }
}

pub fn decide(
    today: NaiveDate,
    current: &str,
    state: &SeasonState,
    seasonal: &[Seasonal],
    restorable: &dyn Fn(&str) -> bool,
) -> Decision {
    let mut state = state.clone();
    let available_today = |id: &str| {
        seasonal
            .iter()
            .find(|s| s.id == id)
            .is_none_or(|s| s.season.available.contains(today))
    };
    let is_seasonal = |id: &str| seasonal.iter().any(|s| s.id == id);

    // 1. A seasonal theme outside its window is locked and gets deselected.
    if is_seasonal(current) && !available_today(current) {
        let back = state
            .previous_theme
            .take()
            .filter(|p| restorable(p) && available_today(p))
            .unwrap_or_else(|| "main".to_string());
        state.notice = Some(StoredNotice::Ended {
            theme_id: current.to_string(),
        });
        drop_stale_selected_notice(&mut state, &back);
        return Decision {
            theme: Some(back),
            state,
        };
    }

    // 2. The first theme whose auto-select window holds today, once a season.
    let mut ordered: Vec<&Seasonal> = seasonal.iter().collect();
    ordered.sort_by(|a, b| a.id.cmp(b.id));
    let pick = ordered.into_iter().find_map(|s| {
        let window = s.season.auto_select.as_ref()?;
        if !window.contains(today) {
            return None;
        }
        // The season is the available window's occurrence, not autoSelect's.
        let season_id = s.season.available.season_id(today)?;
        let done = state.applied_for.get(s.id) == Some(&season_id);
        (!done && !state.opted_out.iter().any(|o| o == s.id)).then_some((s.id, season_id))
    });
    if let Some((id, season_id)) = pick {
        state.applied_for.insert(id.to_string(), season_id);
        if current == id {
            drop_stale_selected_notice(&mut state, current);
            return Decision { theme: None, state };
        }
        state.previous_theme = Some(current.to_string());
        state.notice = Some(StoredNotice::Selected {
            theme_id: id.to_string(),
        });
        return Decision {
            theme: Some(id.to_string()),
            state,
        };
    }

    // 3. Nothing to change.
    drop_stale_selected_notice(&mut state, current);
    Decision { theme: None, state }
}

/// Called when the active theme changes by any route but the season check.
pub fn note_theme_change(state: &mut SeasonState, old: &str, new: &str, seasonal: &[Seasonal]) {
    let is_seasonal = |id: &str| seasonal.iter().any(|s| s.id == id);
    if is_seasonal(new) && !is_seasonal(old) {
        state.previous_theme = Some(old.to_string());
    }
    drop_stale_selected_notice(state, new);
}

/// The "Switch back" button: a lasting no for this theme id.
pub fn switch_back(
    state: &mut SeasonState,
    theme_id: &str,
    restorable: &dyn Fn(&str) -> bool,
) -> String {
    let back = state
        .previous_theme
        .take()
        .filter(|p| restorable(p))
        .unwrap_or_else(|| "main".to_string());
    if !state.opted_out.iter().any(|o| o == theme_id) {
        state.opted_out.push(theme_id.to_string());
    }
    state.notice = None;
    back
}

pub const SEASON_STATE_KEY: &str = "theme_season_state";
pub const SEASON_TODAY_FILE: &str = ".season-today";
/// E2e builds only: when `<runtime>/.season-reset` exists, the next season
/// check deletes it and starts from the default state.
pub const SEASON_RESET_FILE: &str = ".season-reset";

/// The built-in themes that have a season, read from their own manifests.
pub fn builtin_seasonal() -> &'static [(&'static str, ManifestSeason)] {
    static SEASONAL: OnceLock<Vec<(&'static str, ManifestSeason)>> = OnceLock::new();
    SEASONAL.get_or_init(|| {
        let manifest: Value = serde_json::from_str(include_str!(
            "../../../../../packages/ui/themes/silent-night/manifest.json"
        ))
        .expect("the silent-night manifest is valid JSON");
        let mut warnings = Vec::new();
        let season = parse_season(&manifest["season"], None, &mut warnings)
            .unwrap_or_else(|e| panic!("the silent-night season block is invalid: {e:?}"));
        assert!(
            warnings.is_empty(),
            "the silent-night season block has warnings: {warnings:?}"
        );
        vec![("silent-night", season)]
    })
}

/// Today's date. `<runtime>/.season-today` overrides it in every build, for
/// testing seasonal themes outside their season. E2e builds without the file
/// use a fixed off-season day, so no test reads the real clock.
pub fn season_today(runtime: &Path) -> NaiveDate {
    let from_file = std::fs::read_to_string(runtime.join(SEASON_TODAY_FILE))
        .ok()
        .and_then(|t| NaiveDate::parse_from_str(t.trim(), "%Y-%m-%d").ok());
    if let Some(day) = from_file {
        return day;
    }
    #[cfg(feature = "e2e")]
    return NaiveDate::from_ymd_opt(2026, 6, 15).expect("a real date");
    #[cfg(not(feature = "e2e"))]
    chrono::Local::now().date_naive()
}

fn seasonal_list() -> Vec<Seasonal<'static>> {
    builtin_seasonal()
        .iter()
        .map(|(id, season)| Seasonal { id, season })
        .collect()
}

fn mmdd(m: MonthDay) -> String {
    format!("{:02}-{:02}", m.month, m.day)
}

impl ManagerRef<'_, ThemeManager> {
    /// `user:` ids need an installed folder; anything else the renderer maps
    /// to `main` by itself.
    fn restorable_fn(self) -> impl Fn(&str) -> bool {
        let root = self.themes_root();
        move |id: &str| match id.strip_prefix("user:") {
            Some(bare) => theme_dir(&root, bare).is_ok(),
            None => true,
        }
    }

    /// A failed read is an error, so no caller saves a default over the real
    /// row. Only a row that isn't valid JSON falls back to the default.
    async fn load_season_state(self) -> anyhow::Result<SeasonState> {
        super::load_json_pref(&self.app.db, SEASON_STATE_KEY, "theme season state").await
    }

    /// Held for every read-modify-write of the season state, and by
    /// `set_settings` around a theme change. The order is always season, then
    /// the database write.
    pub(crate) async fn lock_season(self) -> tokio::sync::OwnedMutexGuard<()> {
        self.season.clone().lock_owned().await
    }

    async fn store_season_state(self, state: &SeasonState) -> anyhow::Result<()> {
        let text = serde_json::to_string(state)?;
        frontend_preference::upsert_preference(
            &self.app.db,
            SEASON_STATE_KEY,
            &text,
            DbDateTime(Utc::now().fixed_offset()),
        )
        .await?;
        self.app.invalidate(GET_SEASON, None);
        Ok(())
    }

    async fn write_theme(self, theme: &str) -> anyhow::Result<()> {
        self.app
            .settings_manager()
            .set(AppConfigurationPatch {
                theme: Some(theme.to_string()),
                ..Default::default()
            })
            .await?;
        self.set_active(theme);
        self.app.invalidate(GET_SETTINGS, None);
        Ok(())
    }

    pub fn season_today(self) -> NaiveDate {
        let root = self
            .app
            .settings_manager()
            .runtime_path
            .get_root()
            .to_path();
        season_today(&root)
    }

    pub async fn check_season(self) -> anyhow::Result<()> {
        self.check_season_at(self.season_today()).await
    }

    pub async fn check_season_at(self, today: NaiveDate) -> anyhow::Result<()> {
        let _guard = self.lock_season().await;
        #[cfg(feature = "e2e")]
        {
            let reset = self
                .app
                .settings_manager()
                .runtime_path
                .get_root()
                .to_path()
                .join(SEASON_RESET_FILE);
            if reset.exists() {
                std::fs::remove_file(&reset)?;
                self.store_season_state(&SeasonState::default()).await?;
            }
        }
        let current = self.app.settings_manager().get_settings().await?.theme;
        let before = self.load_season_state().await?;
        let restorable = self.restorable_fn();
        let decision = decide(today, &current, &before, &seasonal_list(), &restorable);
        // The state first: if it can't be saved, the theme stays as it was.
        if decision.state != before {
            self.store_season_state(&decision.state).await?;
        }
        if let Some(theme) = &decision.theme {
            self.write_theme(theme).await?;
        }
        Ok(())
    }

    pub async fn season_status(self, today: NaiveDate) -> anyhow::Result<FESeasonStatus> {
        let current = self.app.settings_manager().get_settings().await?.theme;
        let state = self.load_season_state().await?;
        let themes = builtin_seasonal()
            .iter()
            .map(|(id, season)| FESeasonalTheme {
                id: id.to_string(),
                available: season.available.contains(today),
                from: mmdd(season.available.start),
                until: mmdd(season.available.end),
                ends_on: season
                    .available
                    .end_date(today)
                    .map(|d| d.format("%Y-%m-%d").to_string()),
            })
            .collect::<Vec<_>>();
        let notice = match state.notice {
            Some(StoredNotice::Selected { theme_id }) if theme_id == current => themes
                .iter()
                .find(|t| t.id == theme_id)
                .and_then(|t| t.ends_on.clone())
                .map(|ends_on| FESeasonNotice::Selected { theme_id, ends_on }),
            Some(StoredNotice::Selected { .. }) => None,
            Some(StoredNotice::Ended { theme_id }) => Some(FESeasonNotice::Ended { theme_id }),
            None => None,
        };
        Ok(FESeasonStatus {
            today: today.format("%Y-%m-%d").to_string(),
            themes,
            notice,
        })
    }

    pub async fn ack_season_notice(self) -> anyhow::Result<()> {
        let _guard = self.lock_season().await;
        let mut state = self.load_season_state().await?;
        if state.notice.is_some() {
            state.notice = None;
            self.store_season_state(&state).await?;
        }
        Ok(())
    }

    pub async fn season_switch_back(self, theme_id: &str) -> anyhow::Result<()> {
        let _guard = self.lock_season().await;
        let mut state = self.load_season_state().await?;
        let restorable = self.restorable_fn();
        let back = switch_back(&mut state, theme_id, &restorable);
        self.store_season_state(&state).await?;
        self.write_theme(&back).await?;
        Ok(())
    }

    /// Takes the season lock itself, for callers that don't hold it.
    pub async fn record_theme_change(self, old: &str, new: &str) -> anyhow::Result<()> {
        let _guard = self.lock_season().await;
        self.record_theme_change_locked(old, new).await
    }

    /// For `set_settings`, which already holds the guard from `lock_season`.
    pub(crate) async fn record_theme_change_locked(
        self,
        old: &str,
        new: &str,
    ) -> anyhow::Result<()> {
        if old == new {
            return Ok(());
        }
        let before = self.load_season_state().await?;
        let mut state = before.clone();
        note_theme_change(&mut state, old, new, &seasonal_list());
        if state != before {
            self.store_season_state(&state).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use carbon_repos::db_exec::WriteAccess;
    use serde_json::json;

    async fn theme_of(app: &crate::managers::App) -> String {
        app.settings_manager().get_settings().await.unwrap().theme
    }

    async fn set_theme(app: &crate::managers::App, theme: &str) {
        app.settings_manager()
            .set(AppConfigurationPatch {
                theme: Some(theme.into()),
                ..Default::default()
            })
            .await
            .unwrap();
    }

    async fn stored(app: &crate::managers::App) -> SeasonState {
        let row = frontend_preference::get_preference(&app.db, SEASON_STATE_KEY)
            .await
            .unwrap()
            .expect("a stored state");
        serde_json::from_str(&row.value).unwrap()
    }

    #[test]
    fn the_registry_matches_the_packages() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packages/ui/themes");
        let mut seen = 0;
        for entry in std::fs::read_dir(root).unwrap() {
            let entry = entry.unwrap();
            if !entry.file_type().unwrap().is_dir() {
                continue;
            }
            let id = entry.file_name().to_string_lossy().into_owned();
            let text = std::fs::read_to_string(entry.path().join("manifest.json")).unwrap();
            let v: Value = serde_json::from_str(&text).unwrap();
            let registered = builtin_seasonal().iter().find(|(i, _)| *i == id);
            match (v.get("season"), registered) {
                (Some(season), Some((_, expected))) => {
                    let parsed = parse_season(season, None, &mut Vec::new()).unwrap();
                    assert_eq!(&parsed, expected, "{id}");
                }
                (None, None) => {}
                (s, r) => panic!("{id}: season {:?} but registered {:?}", s.is_some(), r),
            }
            seen += 1;
        }
        assert!(seen > 0);
        assert!(
            builtin_seasonal()
                .iter()
                .any(|(id, _)| *id == "silent-night"),
            "silent-night is registered"
        );
    }

    #[tokio::test]
    async fn december_auto_selects_and_writes_the_state() {
        let app = crate::setup_managers_for_test().await;
        set_theme(&app, "frost").await;
        app.theme_manager()
            .check_season_at(d(2026, 12, 1))
            .await
            .unwrap();
        assert_eq!(theme_of(&app).await, "silent-night");
        let status = app
            .theme_manager()
            .season_status(d(2026, 12, 1))
            .await
            .unwrap();
        assert_eq!(
            status.notice,
            Some(FESeasonNotice::Selected {
                theme_id: "silent-night".into(),
                ends_on: "2027-01-31".into()
            })
        );
        assert_eq!(stored(&app).await.previous_theme.as_deref(), Some("frost"));
    }

    #[tokio::test]
    async fn ack_and_switch_back() {
        let app = crate::setup_managers_for_test().await;
        set_theme(&app, "frost").await;
        let themes = app.theme_manager();
        themes.check_season_at(d(2026, 12, 1)).await.unwrap();
        themes.ack_season_notice().await.unwrap();
        let status = themes.season_status(d(2026, 12, 1)).await.unwrap();
        assert_eq!(status.notice, None);

        themes.check_season_at(d(2027, 12, 1)).await.unwrap();
        themes.season_switch_back("silent-night").await.unwrap();
        assert_eq!(theme_of(&app).await, "frost");
        assert_eq!(stored(&app).await.opted_out, vec!["silent-night"]);
        themes.check_season_at(d(2028, 12, 1)).await.unwrap();
        assert_eq!(theme_of(&app).await, "frost");
    }

    #[tokio::test]
    async fn season_end_restores_main_when_the_previous_theme_was_removed() {
        let app = crate::setup_managers_for_test().await;
        set_theme(&app, "user:gone").await;
        let themes = app.theme_manager();
        themes.check_season_at(d(2026, 12, 1)).await.unwrap();
        assert_eq!(theme_of(&app).await, "silent-night");
        themes.check_season_at(d(2027, 2, 1)).await.unwrap();
        assert_eq!(theme_of(&app).await, "main");
        let status = themes.season_status(d(2027, 2, 1)).await.unwrap();
        assert_eq!(
            status.notice,
            Some(FESeasonNotice::Ended {
                theme_id: "silent-night".into()
            })
        );
    }

    #[tokio::test]
    async fn season_end_restores_an_installed_theme_that_still_exists() {
        let app = crate::setup_managers_for_test().await;
        let dir = app.theme_manager().themes_root().join("mine");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("manifest.json"), "{}").unwrap();
        set_theme(&app, "user:mine").await;
        let themes = app.theme_manager();
        themes.check_season_at(d(2026, 12, 1)).await.unwrap();
        assert_eq!(theme_of(&app).await, "silent-night");
        themes.check_season_at(d(2027, 2, 1)).await.unwrap();
        assert_eq!(theme_of(&app).await, "user:mine");
    }

    #[tokio::test]
    async fn a_hand_pick_is_remembered() {
        let app = crate::setup_managers_for_test().await;
        set_theme(&app, "win95").await;
        let update: crate::api::settings::FESettingsUpdate =
            serde_json::from_value(json!({"theme": {"Set": "silent-night"}})).unwrap();
        app.settings_manager().set_settings(update).await.unwrap();
        assert_eq!(theme_of(&app).await, "silent-night");
        app.theme_manager()
            .check_season_at(d(2027, 2, 1))
            .await
            .unwrap();
        assert_eq!(theme_of(&app).await, "win95");
    }

    #[tokio::test]
    async fn status_reports_availability() {
        let app = crate::setup_managers_for_test().await;
        let off = app
            .theme_manager()
            .season_status(d(2026, 6, 15))
            .await
            .unwrap();
        assert_eq!(
            off.themes,
            vec![FESeasonalTheme {
                id: "silent-night".into(),
                available: false,
                from: "12-01".into(),
                until: "01-31".into(),
                ends_on: None
            }]
        );
        let on = app
            .theme_manager()
            .season_status(d(2027, 1, 10))
            .await
            .unwrap();
        assert!(on.themes[0].available);
        assert_eq!(on.themes[0].ends_on.as_deref(), Some("2027-01-31"));
        assert_eq!(on.today, "2027-01-10");
    }

    #[tokio::test]
    async fn a_selected_notice_is_hidden_while_another_theme_is_active() {
        let app = crate::setup_managers_for_test().await;
        set_theme(&app, "win95").await;
        let themes = app.theme_manager();
        themes.check_season_at(d(2026, 12, 1)).await.unwrap();
        assert!(
            themes
                .season_status(d(2026, 12, 1))
                .await
                .unwrap()
                .notice
                .is_some()
        );
        set_theme(&app, "frost").await;
        assert_eq!(
            themes.season_status(d(2026, 12, 1)).await.unwrap().notice,
            None
        );
    }

    #[tokio::test]
    async fn a_failed_read_does_not_erase_the_opt_out() {
        let app = crate::setup_managers_for_test().await;
        set_theme(&app, "frost").await;
        let themes = app.theme_manager();
        themes.check_season_at(d(2026, 12, 1)).await.unwrap();
        themes.season_switch_back("silent-night").await.unwrap();
        assert_eq!(theme_of(&app).await, "frost");
        // Reads of the table now fail; the row itself is renamed, not lost.
        app.db
            .write(|conn| {
                conn.execute_batch(
                    "ALTER TABLE FrontendPreference RENAME TO FrontendPreferenceGone",
                )?;
                Ok(())
            })
            .await
            .unwrap();
        // The read itself fails, so no caller can save a default over the row.
        assert!(themes.load_season_state().await.is_err());
        assert!(themes.check_season_at(d(2027, 12, 1)).await.is_err());
        assert_eq!(theme_of(&app).await, "frost");
        app.db
            .write(|conn| {
                conn.execute_batch(
                    "ALTER TABLE FrontendPreferenceGone RENAME TO FrontendPreference",
                )?;
                Ok(())
            })
            .await
            .unwrap();
        assert_eq!(stored(&app).await.opted_out, vec!["silent-night"]);
        themes.check_season_at(d(2027, 12, 1)).await.unwrap();
        assert_eq!(theme_of(&app).await, "frost");
    }

    #[tokio::test]
    async fn a_broken_state_row_is_the_default_state() {
        let app = crate::setup_managers_for_test().await;
        frontend_preference::upsert_preference(
            &app.db,
            SEASON_STATE_KEY,
            "not json",
            DbDateTime(Utc::now().fixed_offset()),
        )
        .await
        .unwrap();
        set_theme(&app, "frost").await;
        app.theme_manager()
            .check_season_at(d(2026, 12, 1))
            .await
            .unwrap();
        assert_eq!(theme_of(&app).await, "silent-night");
    }

    #[test]
    fn season_today_reads_the_override_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(SEASON_TODAY_FILE), "1999-12-24\n").unwrap();
        assert_eq!(season_today(dir.path()), d(1999, 12, 24));
        std::fs::write(dir.path().join(SEASON_TODAY_FILE), "garbage").unwrap();
        #[cfg(feature = "e2e")]
        assert_eq!(season_today(dir.path()), d(2026, 6, 15));
        #[cfg(not(feature = "e2e"))]
        assert_ne!(season_today(dir.path()), d(1999, 12, 24));
    }

    #[cfg(feature = "e2e")]
    #[tokio::test]
    async fn a_reset_file_clears_the_season_state_once() {
        let app = crate::setup_managers_for_test().await;
        let themes = app.theme_manager();
        let reset = app
            .settings_manager()
            .runtime_path
            .get_root()
            .to_path()
            .join(SEASON_RESET_FILE);
        let opted_out = SeasonState {
            opted_out: vec!["silent-night".into()],
            ..Default::default()
        };
        themes.store_season_state(&opted_out).await.unwrap();
        std::fs::write(&reset, "").unwrap();
        themes.check_season_at(d(2026, 6, 15)).await.unwrap();
        assert_eq!(stored(&app).await, SeasonState::default());
        assert!(!reset.exists());
        themes.store_season_state(&opted_out).await.unwrap();
        themes.check_season_at(d(2026, 6, 15)).await.unwrap();
        assert_eq!(stored(&app).await.opted_out, vec!["silent-night"]);
    }

    #[cfg(feature = "e2e")]
    #[test]
    fn e2e_builds_ignore_the_real_clock() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(season_today(dir.path()), d(2026, 6, 15));
    }

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn md(month: u32, day: u32) -> MonthDay {
        MonthDay { month, day }
    }

    fn sn() -> ManifestSeason {
        ManifestSeason {
            available: SeasonWindow {
                start: md(12, 1),
                end: md(1, 31),
            },
            auto_select: Some(SeasonWindow {
                start: md(12, 1),
                end: md(12, 31),
            }),
            art: None,
        }
    }

    fn all(_: &str) -> bool {
        true
    }

    fn run(today: NaiveDate, current: &str, state: &SeasonState) -> Decision {
        run_with(today, current, state, &all)
    }

    fn run_with(
        today: NaiveDate,
        current: &str,
        state: &SeasonState,
        restorable: &dyn Fn(&str) -> bool,
    ) -> Decision {
        let season = sn();
        let list = [Seasonal {
            id: "silent-night",
            season: &season,
        }];
        decide(today, current, state, &list, restorable)
    }

    fn msg(r: Result<ManifestSeason, ThemeError>) -> String {
        match r {
            Err(ThemeError::Invalid(m)) => m,
            other => panic!("expected Invalid, got {other:?}"),
        }
    }

    fn parse(v: Value) -> Result<ManifestSeason, ThemeError> {
        parse_season(&v, None, &mut Vec::new())
    }

    #[test]
    fn parses_a_season() {
        let mut w = Vec::new();
        let s = parse_season(
            &json!({"available": ["12-01","01-31"], "autoSelect": ["12-01","12-31"], "art": "a.png", "x": 1}),
            None,
            &mut w,
        )
        .unwrap();
        assert_eq!(
            s,
            ManifestSeason {
                art: Some("a.png".into()),
                ..sn()
            }
        );
        assert_eq!(w, vec!["season.x isn't a season key and was ignored"]);

        let files = ThemeFiles {
            files: [("art/a.png".to_string(), 10)].into_iter().collect(),
            fingerprint: 0,
        };
        let s = parse_season(
            &json!({"available": ["12-01","01-31"], "art": "./art/a.png"}),
            Some(&files),
            &mut Vec::new(),
        )
        .unwrap();
        assert_eq!(s.art.as_deref(), Some("art/a.png"));
        assert_eq!(s.auto_select, None);
        assert!(
            parse_season(
                &json!({"available": ["12-01","01-31"], "art": "missing.png"}),
                Some(&files),
                &mut Vec::new()
            )
            .is_err()
        );
    }

    #[test]
    fn refuses_bad_seasons() {
        for bad in ["12-1", "13-01", "02-29", "02-30", "1-01", "12/01", "12-011"] {
            let m = msg(parse(json!({"available": [bad, "12-31"]})));
            assert!(m.contains("season.available"), "{bad}: {m}");
        }
        msg(parse(json!({})));
        msg(parse(json!("12-01")));
        msg(parse(json!({"available": ["12-01"]})));
        msg(parse(json!({"available": ["12-01", 5]})));
        msg(parse(json!({
            "available": ["12-01","01-31"], "autoSelect": ["11-20","12-31"]
        })));
        msg(parse(json!({
            "available": ["12-01","01-31"], "autoSelect": ["12-01","02-01"]
        })));
        // Reversed pair: it would wrap, so it would cover June.
        msg(parse(json!({
            "available": ["12-01","01-31"], "autoSelect": ["12-31","12-01"]
        })));
        // A wrapping autoSelect inside a non-wrapping available.
        msg(parse(json!({
            "available": ["06-01","09-30"], "autoSelect": ["09-01","06-15"]
        })));
        msg(parse(json!({
            "available": ["12-01","01-31"], "autoSelect": ["01-15","01-01"]
        })));
        // These are fine.
        parse(json!({"available": ["12-01","01-31"], "autoSelect": ["01-01","01-15"]})).unwrap();
        parse(json!({"available": ["12-01","01-31"], "autoSelect": ["12-20","01-10"]})).unwrap();
    }

    #[test]
    fn rule_one_does_not_restore_an_unavailable_seasonal_previous_theme() {
        let s = SeasonState {
            previous_theme: Some("silent-night".into()),
            ..Default::default()
        };
        let season = sn();
        let june = ManifestSeason {
            available: SeasonWindow {
                start: md(6, 1),
                end: md(6, 30),
            },
            auto_select: None,
            art: None,
        };
        let list = [
            Seasonal {
                id: "silent-night",
                season: &season,
            },
            Seasonal {
                id: "summer",
                season: &june,
            },
        ];
        let r = decide(d(2027, 2, 1), "summer", &s, &list, &all);
        assert_eq!(r.theme.as_deref(), Some("main"));
        assert_eq!(r.state.previous_theme, None);
    }

    #[test]
    fn a_window_that_does_not_wrap_selects_and_deselects() {
        let june = ManifestSeason {
            available: SeasonWindow {
                start: md(6, 1),
                end: md(6, 30),
            },
            auto_select: Some(SeasonWindow {
                start: md(6, 1),
                end: md(6, 10),
            }),
            art: None,
        };
        let list = [Seasonal {
            id: "summer",
            season: &june,
        }];
        let on = decide(d(2026, 6, 1), "frost", &SeasonState::default(), &list, &all);
        assert_eq!(on.theme.as_deref(), Some("summer"));
        assert_eq!(on.state.applied_for["summer"], "2026");
        let later = decide(
            d(2026, 6, 20),
            "frost",
            &SeasonState::default(),
            &list,
            &all,
        );
        assert_eq!(later.theme, None);
        let off = decide(d(2026, 7, 1), "summer", &on.state, &list, &all);
        assert_eq!(off.theme.as_deref(), Some("frost"));
        assert!(matches!(off.state.notice, Some(StoredNotice::Ended { .. })));
    }

    #[test]
    fn season_boundaries_do_nothing() {
        let s = SeasonState::default();
        let r = run(d(2027, 1, 31), "silent-night", &s);
        assert_eq!(r.theme, None);
        let mut december = SeasonState::default();
        december
            .applied_for
            .insert("silent-night".into(), "2026".into());
        let r = run(d(2027, 1, 1), "silent-night", &december);
        assert_eq!(r.theme, None);
        assert_eq!(r.state, december);
        let r = run(d(2027, 1, 1), "frost", &december);
        assert_eq!(r.theme, None);
    }

    #[test]
    fn the_season_id_follows_the_available_window() {
        let mut jan = sn();
        jan.auto_select = Some(SeasonWindow {
            start: md(1, 1),
            end: md(1, 15),
        });
        let list = [Seasonal {
            id: "silent-night",
            season: &jan,
        }];
        let r = decide(d(2027, 1, 5), "frost", &SeasonState::default(), &list, &all);
        assert_eq!(r.theme.as_deref(), Some("silent-night"));
        assert_eq!(r.state.applied_for["silent-night"], "2026");
        let again = decide(d(2027, 1, 6), "frost", &r.state, &list, &all);
        assert_eq!(again.theme, None);
    }

    #[test]
    fn auto_select_needs_an_available_today() {
        let mut bad = sn();
        bad.auto_select = Some(SeasonWindow {
            start: md(12, 31),
            end: md(12, 1),
        });
        let list = [Seasonal {
            id: "silent-night",
            season: &bad,
        }];
        let r = decide(
            d(2026, 6, 15),
            "frost",
            &SeasonState::default(),
            &list,
            &all,
        );
        assert_eq!(r.theme, None);
    }

    #[test]
    fn windows_wrap_the_new_year() {
        let w = sn().available;
        for day in [
            d(2026, 12, 1),
            d(2026, 12, 31),
            d(2027, 1, 1),
            d(2027, 1, 31),
        ] {
            assert!(w.contains(day), "{day}");
        }
        for day in [d(2026, 11, 30), d(2027, 2, 1), d(2028, 2, 29)] {
            assert!(!w.contains(day), "{day}");
        }
        assert_eq!(w.season_id(d(2027, 1, 15)).as_deref(), Some("2026"));
        assert_eq!(w.season_id(d(2026, 12, 5)).as_deref(), Some("2026"));
        assert_eq!(w.season_id(d(2026, 11, 30)), None);
        assert_eq!(w.end_date(d(2026, 12, 5)), Some(d(2027, 1, 31)));
        assert_eq!(w.end_date(d(2027, 1, 15)), Some(d(2027, 1, 31)));
        assert_eq!(w.end_date(d(2027, 2, 1)), None);

        let june = SeasonWindow {
            start: md(6, 1),
            end: md(6, 30),
        };
        assert!(june.contains(d(2026, 6, 1)) && june.contains(d(2026, 6, 30)));
        assert!(!june.contains(d(2026, 7, 1)) && !june.contains(d(2026, 5, 31)));
        assert_eq!(june.season_id(d(2026, 6, 15)).as_deref(), Some("2026"));
        assert_eq!(june.end_date(d(2026, 6, 15)), Some(d(2026, 6, 30)));
    }

    #[test]
    fn auto_selects_once_in_december() {
        let s = SeasonState::default();
        let r = run(d(2026, 12, 1), "frost", &s);
        assert_eq!(r.theme.as_deref(), Some("silent-night"));
        assert_eq!(r.state.previous_theme.as_deref(), Some("frost"));
        assert_eq!(r.state.applied_for["silent-night"], "2026");
        assert_eq!(
            r.state.notice,
            Some(StoredNotice::Selected {
                theme_id: "silent-night".into()
            })
        );
        let again = run(d(2026, 12, 1), "silent-night", &r.state);
        assert_eq!(again.theme, None);
        assert_eq!(again.state, r.state);
        let left = run(d(2026, 12, 10), "aether", &r.state);
        assert_eq!(left.theme, None);
        assert_eq!(left.state.notice, None);
    }

    #[test]
    fn already_on_the_theme_only_records_the_season() {
        let r = run(d(2026, 12, 3), "silent-night", &SeasonState::default());
        assert_eq!(r.theme, None);
        assert_eq!(r.state.applied_for["silent-night"], "2026");
        assert_eq!(r.state.previous_theme, None);
        assert_eq!(r.state.notice, None);
    }

    #[test]
    fn january_has_no_auto_select() {
        let r = run(d(2027, 1, 5), "frost", &SeasonState::default());
        assert_eq!(r.theme, None);
        assert_eq!(r.state, SeasonState::default());
    }

    #[test]
    fn season_end_restores_the_previous_theme() {
        let s = SeasonState {
            previous_theme: Some("frost".into()),
            ..Default::default()
        };
        let r = run(d(2027, 2, 1), "silent-night", &s);
        assert_eq!(r.theme.as_deref(), Some("frost"));
        assert_eq!(r.state.previous_theme, None);
        assert_eq!(
            r.state.notice,
            Some(StoredNotice::Ended {
                theme_id: "silent-night".into()
            })
        );
    }

    #[test]
    fn startup_outside_the_season_deselects() {
        let r = run(d(2026, 11, 30), "silent-night", &SeasonState::default());
        assert_eq!(r.theme.as_deref(), Some("main"));
        assert!(matches!(r.state.notice, Some(StoredNotice::Ended { .. })));
    }

    #[test]
    fn falls_back_to_main_when_previous_is_not_restorable() {
        let s = SeasonState {
            previous_theme: Some("frost".into()),
            ..Default::default()
        };
        let r = run_with(d(2027, 2, 1), "silent-night", &s, &|_| false);
        assert_eq!(r.theme.as_deref(), Some("main"));
        assert_eq!(r.state.previous_theme, None);
    }

    #[test]
    fn opted_out_themes_never_auto_select() {
        let s = SeasonState {
            opted_out: vec!["silent-night".into()],
            ..Default::default()
        };
        let r = run(d(2027, 12, 1), "frost", &s);
        assert_eq!(r.theme, None);
    }

    #[test]
    fn a_new_season_selects_again() {
        let mut s = SeasonState::default();
        s.applied_for.insert("silent-night".into(), "2026".into());
        let r = run(d(2027, 12, 2), "frost", &s);
        assert_eq!(r.theme.as_deref(), Some("silent-night"));
        assert_eq!(r.state.applied_for["silent-night"], "2027");
    }

    #[test]
    fn manual_pick_records_the_previous_theme() {
        let season = sn();
        let list = [Seasonal {
            id: "silent-night",
            season: &season,
        }];
        let mut s = SeasonState::default();
        note_theme_change(&mut s, "win95", "silent-night", &list);
        assert_eq!(s.previous_theme.as_deref(), Some("win95"));
        note_theme_change(&mut s, "silent-night", "frost", &list);
        assert_eq!(s.previous_theme.as_deref(), Some("win95"));

        s.notice = Some(StoredNotice::Selected {
            theme_id: "silent-night".into(),
        });
        note_theme_change(&mut s, "silent-night", "frost", &list);
        assert_eq!(s.notice, None);
    }

    #[test]
    fn switch_back_opts_out_for_good() {
        let mut s = SeasonState {
            previous_theme: Some("frost".into()),
            notice: Some(StoredNotice::Selected {
                theme_id: "silent-night".into(),
            }),
            ..Default::default()
        };
        assert_eq!(switch_back(&mut s, "silent-night", &all), "frost");
        assert_eq!(s.opted_out, vec!["silent-night"]);
        assert_eq!(s.notice, None);
        assert_eq!(s.previous_theme, None);
        assert_eq!(switch_back(&mut s, "silent-night", &all), "main");
        assert_eq!(s.opted_out, vec!["silent-night"]);
    }

    #[test]
    fn a_selected_notice_disappears_when_the_theme_is_left() {
        let s = SeasonState {
            notice: Some(StoredNotice::Selected {
                theme_id: "silent-night".into(),
            }),
            ..Default::default()
        };
        let r = run(d(2027, 1, 10), "frost", &s);
        assert_eq!(r.theme, None);
        assert_eq!(r.state.notice, None);
    }

    #[test]
    fn state_round_trips_as_camel_case_json() {
        let s = SeasonState {
            previous_theme: Some("frost".into()),
            notice: Some(StoredNotice::Ended {
                theme_id: "silent-night".into(),
            }),
            ..Default::default()
        };
        let text = serde_json::to_string(&s).unwrap();
        assert!(text.contains("\"previousTheme\""), "{text}");
        assert!(text.contains("\"kind\":\"ended\""), "{text}");
        assert!(text.contains("\"themeId\""), "{text}");
        assert_eq!(serde_json::from_str::<SeasonState>(&text).unwrap(), s);
        assert_eq!(
            serde_json::from_str::<SeasonState>("{}").unwrap(),
            SeasonState::default()
        );
    }
}
