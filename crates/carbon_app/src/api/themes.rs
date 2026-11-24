use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use rspc::RouterBuilder;
use serde::{Deserialize, Serialize};
use specta::Type;

use super::keys::themes::*;
use super::router::router;
use crate::error::FeErrorCode;
use crate::managers::themes::error::ThemeError;
use crate::managers::themes::extras::{Effect, Length};
use crate::managers::themes::files::{clean_text, is_blank_text, is_valid_theme_id};
use crate::managers::themes::preferences::ThemePreferences;
use crate::managers::themes::streams::{AssetStreams, StreamTicket};
use crate::managers::themes::{AssetUrlBase, InspectedTheme, LoadedTheme, resolve_asset_path};
use crate::managers::{App, AppInner};

pub(super) fn mount() -> RouterBuilder<App> {
    router! {
        query GET_THEME[app, args: FEThemeId] {
            let base = AssetUrlBase::from_server();
            app.theme_manager()
                .get_theme(&args.id, &base)
                .await
                .map(FEResolvedTheme::from)
        }
        query LIST[app, _args: ()] {
            let base = AssetUrlBase::from_server();
            Ok(app
                .theme_manager()
                .list(&base)
                .await?
                .into_iter()
                .map(|(id, loaded)| FEInstalledTheme::from_listing(id, loaded))
                .collect::<Vec<_>>())
        }
        mutation INSTALL[app, args: FEThemeInstall] {
            app.theme_manager().install(PathBuf::from(args.path)).await
        }
        mutation REMOVE[app, args: FEThemeId] {
            app.theme_manager().remove(&args.id).await
        }
        mutation INSPECT[app, args: FEThemeInstall] {
            app.theme_manager()
                .inspect(PathBuf::from(args.path))
                .await
                .map(FEThemePreview::from)
        }
        mutation OPEN_FOLDER[app, args: FEThemeFolder] {
            app.theme_manager().open_folder(args.id).await
        }
        query GET_PREFERENCES[app, _args: ()] {
            app.theme_manager().preferences().await.map(FEThemePreferences::from)
        }
        mutation SET_PREFERENCES[app, args: FEThemePreferencesPatch] {
            app.theme_manager()
                .set_preferences(args)
                .await
                .map(FEThemePreferences::from)
        }
        query GET_SEASON[app, _args: ()] {
            let themes = app.theme_manager();
            themes.season_status(themes.season_today()).await
        }
        mutation CHECK_SEASON[app, _args: ()] {
            let themes = app.theme_manager();
            let today = themes.season_today();
            let result = themes.check_season_at(today).await;
            // today, available and endsOn follow the date, not only the state.
            app.invalidate(GET_SEASON, None);
            result?;
            themes.season_status(today).await
        }
        mutation ACK_SEASON_NOTICE[app, _args: ()] {
            app.theme_manager().ack_season_notice().await
        }
        mutation SEASON_SWITCH_BACK[app, args: FEThemeId] {
            app.theme_manager().season_switch_back(&args.id).await
        }
    }
}

/// Dates are `YYYY-MM-DD`; `from` and `until` are `MM-DD`.
#[derive(Type, Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FESeasonStatus {
    pub today: String,
    pub themes: Vec<FESeasonalTheme>,
    pub notice: Option<FESeasonNotice>,
}

#[derive(Type, Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FESeasonalTheme {
    pub id: String,
    pub available: bool,
    pub from: String,
    pub until: String,
    /// Set while the theme is available.
    pub ends_on: Option<String>,
}

#[derive(Type, Serialize, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum FESeasonNotice {
    #[serde(rename_all = "camelCase")]
    Selected { theme_id: String, ends_on: String },
    #[serde(rename_all = "camelCase")]
    Ended { theme_id: String },
}

#[derive(Type, Deserialize, Debug)]
pub struct FEThemeInstall {
    pub path: String,
}

#[derive(Type, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FEThemePreview {
    pub id: String,
    pub name: String,
    pub author: String,
    pub version: String,
    pub description: Option<String>,
    pub names: BTreeMap<String, String>,
    pub descriptions: BTreeMap<String, String>,
    /// A `data:` URL, left out for a large or missing preview.
    pub preview: Option<String>,
    pub installed: bool,
}

impl From<InspectedTheme> for FEThemePreview {
    fn from(t: InspectedTheme) -> Self {
        Self {
            id: t.manifest.id,
            name: t.manifest.name,
            author: t.manifest.author,
            version: t.manifest.version,
            description: t.manifest.description,
            names: t.manifest.names,
            descriptions: t.manifest.descriptions,
            preview: t.preview,
            installed: t.installed,
        }
    }
}

/// `id: None` is the themes folder itself.
#[derive(Type, Deserialize, Debug)]
pub struct FEThemeFolder {
    pub id: Option<String>,
}

#[derive(Type, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FEThemePreferences {
    pub sounds: bool,
    pub effects: bool,
    pub tile_size_chosen: bool,
}

impl From<ThemePreferences> for FEThemePreferences {
    fn from(p: ThemePreferences) -> Self {
        Self {
            sounds: p.sounds,
            effects: p.effects,
            tile_size_chosen: p.tile_size_chosen,
        }
    }
}

/// Only the fields that are set change.
#[derive(Type, Deserialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FEThemePreferencesPatch {
    #[specta(optional)]
    pub sounds: Option<bool>,
    #[specta(optional)]
    pub effects: Option<bool>,
    #[specta(optional)]
    pub tile_size_chosen: Option<bool>,
}

#[derive(Type, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FEInstalledTheme {
    /// The folder's real name: what open-folder and remove take.
    pub id: String,
    /// The manifest's name, or for a theme that can't be read, its cleaned
    /// folder name.
    pub name: Option<String>,
    pub names: BTreeMap<String, String>,
    pub author: Option<String>,
    pub version: Option<String>,
    pub preview_url: Option<String>,
    /// The theme's `darkSlate-900`, `darkSlate-700` and `darkSlate-600`, when it sets them.
    pub preview_colors: BTreeMap<String, String>,
    pub error: Option<String>,
    /// `ThemeError::error_code()`; none for other errors.
    pub error_code: Option<String>,
    /// False for a folder whose name isn't a valid theme id: it can't be removed.
    pub removable: bool,
}

const PREVIEW_COLOR_TOKENS: [&str; 3] = ["darkSlate-900", "darkSlate-700", "darkSlate-600"];

/// The name a theme that can't be read is listed under: its folder name, which
/// anyone can choose, cleaned like any other untrusted display text. A name
/// with nothing visible left shows as U+FFFD, so the tile is never blank.
fn folder_display_name(id: &str) -> String {
    let name = clean_text(id);
    if is_blank_text(&name) {
        "\u{FFFD}".to_string()
    } else {
        name
    }
}

impl FEInstalledTheme {
    fn from_listing(id: String, loaded: Result<LoadedTheme, ThemeError>) -> Self {
        let removable = is_valid_theme_id(&id);
        match loaded {
            Ok(t) => Self {
                id,
                name: Some(t.manifest.name),
                names: t.manifest.names,
                author: Some(t.manifest.author),
                version: Some(t.manifest.version),
                preview_url: t.preview_url,
                preview_colors: t
                    .manifest
                    .colors
                    .into_iter()
                    .filter(|(k, _)| PREVIEW_COLOR_TOKENS.contains(&k.as_str()))
                    .collect(),
                error: None,
                error_code: None,
                removable,
            },
            Err(e) => Self {
                name: Some(folder_display_name(&id)),
                id,
                names: BTreeMap::new(),
                author: None,
                version: None,
                preview_url: None,
                preview_colors: BTreeMap::new(),
                error: Some(e.to_string()),
                error_code: Some(e.error_code().to_string()),
                removable,
            },
        }
    }
}

/// Stops an SVG used through `mask`, `filter` or `clip-path` from fetching
/// anything outside the theme.
const ASSET_CSP: &str = "default-src 'none'; style-src 'unsafe-inline'; img-src 'self' data:";

#[derive(Deserialize)]
struct AssetQuery {
    id: String,
    path: String,
}

pub(super) fn mount_axum_router() -> axum::Router<Arc<AppInner>> {
    axum::Router::new().route(
        "/asset",
        axum::routing::get(
            |State(app): State<Arc<AppInner>>,
             Query(query): Query<AssetQuery>,
             headers: HeaderMap| async move {
                let themes = app.theme_manager();
                asset_response(
                    themes.themes_root(),
                    themes.asset_streams(),
                    query.id,
                    query.path,
                    &headers,
                )
                .await
            },
        ),
    )
}

/// What a `Range` header asks for.
enum RangeRequest {
    /// No usable range: send the whole file.
    Whole,
    Slice(u64, u64),
    Unsatisfiable,
}

fn parse_range(value: &str, len: u64) -> RangeRequest {
    let Some(spec) = value.trim().strip_prefix("bytes=") else {
        return RangeRequest::Whole;
    };
    if spec.contains(',') {
        return RangeRequest::Whole;
    }
    let Some((from, to)) = spec.trim().split_once('-') else {
        return RangeRequest::Whole;
    };
    let (from, to) = (from.trim(), to.trim());
    let digits = |t: &str| !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit());
    if from.is_empty() {
        if !digits(to) {
            return RangeRequest::Whole;
        }
        let Ok(n) = to.parse::<u64>() else {
            return RangeRequest::Whole;
        };
        if n == 0 || len == 0 {
            return RangeRequest::Unsatisfiable;
        }
        return RangeRequest::Slice(len.saturating_sub(n), len - 1);
    }
    if !digits(from) || !(to.is_empty() || digits(to)) {
        return RangeRequest::Whole;
    }
    let (Ok(a), b) = (
        from.parse::<u64>(),
        if to.is_empty() {
            Ok(u64::MAX)
        } else {
            to.parse::<u64>()
        },
    ) else {
        return RangeRequest::Whole;
    };
    let Ok(b) = b else {
        return RangeRequest::Whole;
    };
    if a > b || a >= len {
        return RangeRequest::Unsatisfiable;
    }
    RangeRequest::Slice(a, b.min(len - 1))
}

fn etag_matches(header_value: &str, etag: &str) -> bool {
    header_value.split(',').any(|candidate| {
        let candidate = candidate.trim();
        candidate == "*" || candidate.strip_prefix("W/").unwrap_or(candidate) == etag
    })
}

async fn asset_response(
    root: PathBuf,
    streams: Arc<AssetStreams>,
    id: String,
    path: String,
    headers: &HeaderMap,
) -> Response {
    let resolved = {
        let id = id.clone();
        tokio::task::spawn_blocking(move || resolve_asset_path(&root, &id, &path)).await
    };
    let (file, content_type) = match resolved {
        Ok(Ok(found)) => found,
        Ok(Err(ThemeError::NotFound(_))) => return StatusCode::NOT_FOUND.into_response(),
        Ok(Err(e)) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    // Taken before the file is opened, so an install or remove of this theme
    // either turns this request away or waits for it to close the file.
    let ticket = streams.ticket(&id);
    if ticket.is_cancelled() {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    let Ok(mut handle) = tokio::fs::File::open(&file).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Ok(meta) = handle.metadata().await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let len = meta.len();
    let mtime = meta.modified().unwrap_or(std::time::UNIX_EPOCH);
    let nanos = mtime
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let etag = format!("\"{len:x}-{nanos:x}\"");

    let mut common = HeaderMap::new();
    common.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    if let Ok(v) = HeaderValue::from_str(&etag) {
        common.insert(header::ETAG, v);
    }
    if let Ok(v) = HeaderValue::from_str(&httpdate::fmt_http_date(mtime)) {
        common.insert(header::LAST_MODIFIED, v);
    }
    common.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    common.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    common.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(ASSET_CSP),
    );

    let header_str = |name| headers.get(name).and_then(|v| v.to_str().ok());
    let not_modified = if let Some(inm) = header_str(header::IF_NONE_MATCH) {
        etag_matches(inm, &etag)
    } else if let Some(since) =
        header_str(header::IF_MODIFIED_SINCE).and_then(|v| httpdate::parse_http_date(v).ok())
    {
        let secs = |t: std::time::SystemTime| {
            t.duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        };
        secs(since) >= secs(mtime)
    } else {
        false
    };
    if not_modified {
        return (StatusCode::NOT_MODIFIED, common).into_response();
    }

    let range_applies = header_str(header::IF_RANGE).is_none_or(|v| v.trim() == etag);
    let range = match header_str(header::RANGE) {
        Some(v) if range_applies => parse_range(v, len),
        _ => RangeRequest::Whole,
    };
    let content_type = (header::CONTENT_TYPE, HeaderValue::from_static(content_type));

    match range {
        RangeRequest::Unsatisfiable => {
            let mut resp = (StatusCode::RANGE_NOT_SATISFIABLE, common).into_response();
            if let Ok(v) = HeaderValue::from_str(&format!("bytes */{len}")) {
                resp.headers_mut().insert(header::CONTENT_RANGE, v);
            }
            resp
        }
        RangeRequest::Slice(a, b) => {
            use tokio::io::AsyncSeekExt;
            if handle.seek(std::io::SeekFrom::Start(a)).await.is_err() {
                return StatusCode::NOT_FOUND.into_response();
            }
            let count = b - a + 1;
            let mut resp = (
                StatusCode::PARTIAL_CONTENT,
                common,
                [content_type],
                stream_body(handle, count, ticket),
            )
                .into_response();
            let headers = resp.headers_mut();
            if let Ok(v) = HeaderValue::from_str(&format!("bytes {a}-{b}/{len}")) {
                headers.insert(header::CONTENT_RANGE, v);
            }
            headers.insert(header::CONTENT_LENGTH, HeaderValue::from(count));
            resp
        }
        RangeRequest::Whole => {
            let mut resp = (
                StatusCode::OK,
                common,
                [content_type],
                stream_body(handle, len, ticket),
            )
                .into_response();
            resp.headers_mut()
                .insert(header::CONTENT_LENGTH, HeaderValue::from(len));
            resp
        }
    }
}

/// How much of the file is read at a time.
const STREAM_CHUNK: u64 = 65_536;

/// The next `count` bytes of the file, read as the client takes them rather
/// than all at once. A task of its own reads the file, so when the theme is
/// replaced or removed (the ticket is cancelled) the file is closed at once,
/// even while the client isn't reading (a paused `<video>`), and the body ends
/// early.
fn stream_body(handle: tokio::fs::File, count: u64, ticket: StreamTicket) -> Body {
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    tokio::spawn(pump_file(handle, count, ticket, tx));
    Body::from_stream(futures::stream::poll_fn(move |cx| rx.poll_recv(cx)))
}

async fn pump_file(
    mut file: tokio::fs::File,
    mut left: u64,
    ticket: StreamTicket,
    tx: tokio::sync::mpsc::Sender<std::io::Result<Vec<u8>>>,
) {
    use tokio::io::AsyncReadExt;
    while left > 0 {
        let mut buf = vec![0; left.min(STREAM_CHUNK) as usize];
        let read = tokio::select! {
            biased;
            () = ticket.cancelled() => break,
            read = file.read(&mut buf) => read,
        };
        let chunk = match read {
            // The file got shorter since it was opened.
            Ok(0) => break,
            Ok(n) => {
                buf.truncate(n);
                left -= n as u64;
                Ok(buf)
            }
            Err(e) => Err(e),
        };
        let failed = chunk.is_err();
        let sent = tokio::select! {
            biased;
            () = ticket.cancelled() => break,
            sent = tx.send(chunk) => sent.is_ok(),
        };
        if failed || !sent {
            break;
        }
    }
    // `into_std` waits out a read still in flight, so the file is really
    // closed before the ticket goes.
    drop(file.into_std().await);
    drop(ticket);
}

#[derive(Type, Deserialize, Debug)]
pub struct FEThemeId {
    pub id: String,
}

#[derive(Type, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FEThemeFont {
    pub family: String,
    pub url: String,
    pub weight: Option<String>,
    pub style: Option<String>,
}

#[derive(Type, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FEThemeTokens {
    pub font: Option<String>,
    pub font_mono: Option<String>,
    pub radius_scale: Option<f64>,
}

#[derive(Type, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FEResolvedTheme {
    pub id: String,
    pub name: String,
    pub author: String,
    pub version: String,
    pub description: Option<String>,
    pub names: BTreeMap<String, String>,
    pub descriptions: BTreeMap<String, String>,
    pub preview_url: Option<String>,
    pub colors: BTreeMap<String, String>,
    pub fonts: Vec<FEThemeFont>,
    pub tokens: FEThemeTokens,
    pub css: Option<String>,
    pub background: Option<FEThemeBackground>,
    pub decorations: Vec<FEThemeDecoration>,
    pub effects: Vec<FEThemeEffect>,
    pub layout: FEThemeLayout,
    pub sounds: FEThemeSounds,
    pub warnings: Vec<String>,
}

#[derive(Type, Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct FEThemeLayout {
    pub nav: Option<String>,
    pub panels: Option<bool>,
    pub tile_style: Option<String>,
    pub tile_size: Option<u8>,
}

/// Asset URLs, one per event.
#[derive(Type, Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct FEThemeSounds {
    pub launch: Option<String>,
    pub download_complete: Option<String>,
    pub click: Option<String>,
    pub notification: Option<String>,
    pub error: Option<String>,
}

#[derive(Type, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FEThemeBackground {
    pub src: String,
    pub video: bool,
    pub motion: Vec<String>,
    pub fit: Option<String>,
    pub position: Option<String>,
    pub dim: Option<f64>,
    pub blur: Option<f64>,
}

/// Pixels as a number, a percentage as `"<n>%"`.
#[derive(Type, Serialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum FELength {
    Px(f64),
    Percent(String),
}

impl From<Length> for FELength {
    fn from(l: Length) -> Self {
        match l {
            Length::Px(n) => FELength::Px(n),
            Length::Percent(n) => FELength::Percent(format!("{n}%")),
        }
    }
}

#[derive(Type, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FEThemeDecoration {
    pub anchor: String,
    pub edge: String,
    pub x: Option<FELength>,
    pub y: Option<FELength>,
    pub width: Option<FELength>,
    pub height: Option<FELength>,
    pub images: Vec<String>,
    pub pick: Option<String>,
    pub repeat: Option<String>,
}

#[derive(Type, Serialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum FEThemeEffect {
    #[serde(rename_all = "camelCase")]
    Snow {
        density: Option<f64>,
        speed: Option<f64>,
        min_size: Option<f64>,
        max_size: Option<f64>,
        wind: Option<f64>,
        color: Option<String>,
        layers: Option<u8>,
    },
    #[serde(rename_all = "camelCase")]
    StringLights {
        colors: Vec<String>,
        spacing: Option<f64>,
        droop: Option<f64>,
        size: Option<f64>,
        twinkle: Option<f64>,
        bulb: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    Particles {
        images: Vec<String>,
        preset: String,
        density: Option<f64>,
        speed: Option<f64>,
        min_size: Option<f64>,
        max_size: Option<f64>,
        rotate: Option<bool>,
        opacity: Option<f64>,
    },
}

impl FEThemeEffect {
    /// `url` maps a path in the theme to its asset URL.
    fn from_effect(e: Effect, url: &dyn Fn(&str) -> String) -> Self {
        match e {
            Effect::Snow {
                density,
                speed,
                min_size,
                max_size,
                wind,
                color,
                layers,
            } => FEThemeEffect::Snow {
                density,
                speed,
                min_size,
                max_size,
                wind,
                color,
                layers,
            },
            Effect::StringLights {
                colors,
                spacing,
                droop,
                size,
                twinkle,
                bulb,
            } => FEThemeEffect::StringLights {
                colors,
                spacing,
                droop,
                size,
                twinkle,
                bulb,
            },
            Effect::Particles {
                images,
                preset,
                density,
                speed,
                min_size,
                max_size,
                rotate,
                opacity,
            } => FEThemeEffect::Particles {
                images: images.iter().map(|p| url(p)).collect(),
                preset,
                density,
                speed,
                min_size,
                max_size,
                rotate,
                opacity,
            },
        }
    }
}

impl From<LoadedTheme> for FEResolvedTheme {
    fn from(t: LoadedTheme) -> Self {
        let m = t.manifest;
        let urls = t.asset_urls;
        // The loader maps every path the manifest names, so a miss is a bug.
        let url = |rel: &str| {
            let found = urls.get(rel).cloned();
            debug_assert!(found.is_some(), "no asset URL for \"{rel}\"");
            found.unwrap_or_default()
        };
        let background = m.background.map(|b| FEThemeBackground {
            src: url(&b.src),
            video: b.video,
            motion: b.motion.iter().map(|p| url(p)).collect(),
            fit: b.fit,
            position: b.position,
            dim: b.dim,
            blur: b.blur,
        });
        let decorations = m
            .decorations
            .into_iter()
            .map(|d| FEThemeDecoration {
                anchor: d.anchor,
                edge: d.edge,
                x: d.x.map(Into::into),
                y: d.y.map(Into::into),
                width: d.width.map(Into::into),
                height: d.height.map(Into::into),
                images: d.images.iter().map(|p| url(p)).collect(),
                pick: d.pick,
                repeat: d.repeat,
            })
            .collect();
        let effects = m
            .effects
            .into_iter()
            .map(|e| FEThemeEffect::from_effect(e, &url))
            .collect();
        let sound = |p: &Option<String>| p.as_deref().map(url);
        let sounds = FEThemeSounds {
            launch: sound(&m.sounds.launch),
            download_complete: sound(&m.sounds.download_complete),
            click: sound(&m.sounds.click),
            notification: sound(&m.sounds.notification),
            error: sound(&m.sounds.error),
        };
        let layout = FEThemeLayout {
            nav: m.layout.nav,
            panels: m.layout.panels,
            tile_style: m.layout.tile_style,
            tile_size: m.layout.tile_size,
        };
        let fonts = m
            .fonts
            .into_iter()
            .zip(t.font_urls)
            .map(|(f, url)| FEThemeFont {
                family: f.family,
                url,
                weight: f.weight,
                style: f.style,
            })
            .collect();
        Self {
            id: m.id,
            name: m.name,
            author: m.author,
            version: m.version,
            description: m.description,
            names: m.names,
            descriptions: m.descriptions,
            preview_url: t.preview_url,
            colors: m.colors,
            fonts,
            tokens: FEThemeTokens {
                font: m.tokens.font,
                font_mono: m.tokens.font_mono,
                radius_scale: m.tokens.radius_scale,
            },
            css: t.css,
            background,
            decorations,
            effects,
            layout,
            sounds,
            warnings: m.warnings,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fe_lengths_and_effects_serialize_as_documented() {
        let d = FEThemeDecoration {
            anchor: "nav".into(),
            edge: "top".into(),
            x: Some(FELength::Px(12.0)),
            y: Some(Length::Percent(50.0).into()),
            width: None,
            height: Some(FELength::Px(5.0)),
            images: vec!["u".into()],
            pick: None,
            repeat: None,
        };
        let v = serde_json::to_value(&d).unwrap();
        assert_eq!(v["x"], 12.0);
        assert_eq!(v["y"], "50%");
        let e = FEThemeEffect::Snow {
            density: Some(1.0),
            speed: None,
            min_size: Some(0.5),
            max_size: None,
            wind: None,
            color: None,
            layers: Some(2),
        };
        let v = serde_json::to_value(&e).unwrap();
        assert_eq!(v["type"], "snow");
        assert_eq!(v["minSize"], 0.5);
        let l = FEThemeEffect::StringLights {
            colors: vec![],
            spacing: None,
            droop: None,
            size: None,
            twinkle: Some(0.5),
            bulb: None,
        };
        assert_eq!(serde_json::to_value(&l).unwrap()["type"], "stringLights");
    }

    #[test]
    fn fe_layout_sounds_and_particles_serialize_as_documented() {
        let l = FEThemeLayout {
            nav: Some("rail".into()),
            panels: Some(false),
            tile_style: Some("caption".into()),
            tile_size: Some(4),
        };
        let v = serde_json::to_value(&l).unwrap();
        assert_eq!(v["tileStyle"], "caption");
        assert_eq!(v["tileSize"], 4);
        let s = FEThemeSounds {
            download_complete: Some("u".into()),
            ..Default::default()
        };
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["downloadComplete"], "u");
        assert!(v["launch"].is_null());
        let p = FEThemeEffect::Particles {
            images: vec!["u".into()],
            preset: "fall".into(),
            density: None,
            speed: None,
            min_size: Some(8.0),
            max_size: None,
            rotate: Some(true),
            opacity: None,
        };
        let v = serde_json::to_value(&p).unwrap();
        assert_eq!(v["type"], "particles");
        assert_eq!(v["minSize"], 8.0);
        assert_eq!(v["images"][0], "u");
    }

    fn write_theme(root: &std::path::Path, folder: &str, manifest: &str) {
        let dir = root.join(folder);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("manifest.json"), manifest).unwrap();
    }

    fn listed(root: &std::path::Path) -> Vec<FEInstalledTheme> {
        let base = AssetUrlBase {
            origin: "http://127.0.0.1:0".to_string(),
            token: None,
        };
        crate::managers::themes::list_theme_dirs(root, &base)
            .unwrap()
            .into_iter()
            .map(|(id, loaded)| FEInstalledTheme::from_listing(id, loaded))
            .collect()
    }

    #[test]
    fn lists_misnamed_folders_with_an_error_code() {
        let root = tempfile::tempdir().unwrap();
        let manifest = |id: &str, extra: &str| {
            format!(
                r#"{{"themeApi":1,"id":"{id}","name":"N","author":"a","version":"1.0.0"{extra}}}"#
            )
        };
        write_theme(root.path(), "My Theme", &manifest("my-theme", ""));
        write_theme(
            root.path(),
            "demo",
            &manifest(
                "demo",
                r#","names":{"italian":"Neve"},"colors":{"darkSlate-900":"1 2 3","primary":"4 5 6"}"#,
            ),
        );
        write_theme(root.path(), "broken", "{ not json");
        let all = listed(root.path());
        let by = |id: &str| all.iter().find(|t| t.id == id).unwrap();

        let misnamed = by("My Theme");
        assert_eq!(misnamed.error_code.as_deref(), Some("THEME_INVALID"));
        assert!(!misnamed.removable);
        assert!(misnamed.error.as_deref().unwrap().contains("folder name"));

        let demo = by("demo");
        assert_eq!(demo.error_code, None);
        assert!(demo.removable);
        assert_eq!(demo.names["italian"], "Neve");
        assert_eq!(
            demo.preview_colors.keys().collect::<Vec<_>>(),
            ["darkSlate-900"]
        );

        let broken = by("broken");
        assert_eq!(broken.error_code.as_deref(), Some("THEME_INVALID"));
        assert!(broken.removable);
    }

    #[test]
    fn broken_folders_are_named_by_their_cleaned_folder_name() {
        let root = tempfile::tempdir().unwrap();
        let bidi = "My\u{202E}Theme\u{200B} v1";
        let joined = "\u{1F43B}\u{200D}\u{2744}\u{FE0F} bear";
        write_theme(root.path(), bidi, "{ not json");
        write_theme(root.path(), joined, "{ not json");
        write_theme(root.path(), "\u{202E}\u{200B}", "{ not json");
        let all = listed(root.path());
        let by = |id: &str| all.iter().find(|t| t.id == id).unwrap();

        // the id stays the folder's real name, so open and remove find it
        assert_eq!(by(bidi).name.as_deref(), Some("MyTheme v1"));
        assert_eq!(by(joined).name.as_deref(), Some(joined));
        assert_eq!(by("\u{202E}\u{200B}").name.as_deref(), Some("\u{FFFD}"));
    }

    fn root() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("demo/assets");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.png"), "png").unwrap();
        std::fs::write(dir.join("a.svg"), "<svg/>").unwrap();
        root
    }

    async fn get(root: &tempfile::TempDir, id: &str, path: &str) -> Response {
        get_from(root, &Arc::default(), id, path).await
    }

    async fn get_from(
        root: &tempfile::TempDir,
        streams: &Arc<AssetStreams>,
        id: &str,
        path: &str,
    ) -> Response {
        asset_response(
            root.path().to_path_buf(),
            streams.clone(),
            id.into(),
            path.into(),
            &HeaderMap::new(),
        )
        .await
    }

    async fn get_with(root: &tempfile::TempDir, headers: &[(&str, &str)]) -> Response {
        let mut map = HeaderMap::new();
        for (k, v) in headers {
            map.insert(
                axum::http::HeaderName::from_bytes(k.as_bytes()).unwrap(),
                HeaderValue::from_str(v).unwrap(),
            );
        }
        asset_response(
            root.path().to_path_buf(),
            Arc::default(),
            "demo".into(),
            "assets/a.png".into(),
            &map,
        )
        .await
    }

    fn digits() -> tempfile::TempDir {
        let root = root();
        std::fs::write(root.path().join("demo/assets/a.png"), "0123456789").unwrap();
        root
    }

    async fn body(r: Response) -> Vec<u8> {
        axum::body::to_bytes(r.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec()
    }

    #[tokio::test]
    async fn serves_a_byte_range() {
        let root = digits();
        let r = get_with(&root, &[("range", "bytes=2-5")]).await;
        assert_eq!(r.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(r.headers()[header::CONTENT_RANGE], "bytes 2-5/10");
        assert_eq!(r.headers()[header::ACCEPT_RANGES], "bytes");
        assert_eq!(body(r).await, b"2345");
    }

    #[tokio::test]
    async fn serves_open_and_suffix_ranges() {
        let root = digits();
        for (range, want) in [
            ("bytes=7-", "789"),
            ("bytes=-3", "789"),
            ("bytes=8-100", "89"),
        ] {
            let r = get_with(&root, &[("range", range)]).await;
            assert_eq!(r.status(), StatusCode::PARTIAL_CONTENT, "{range}");
            assert_eq!(body(r).await, want.as_bytes(), "{range}");
        }
    }

    #[tokio::test]
    async fn refuses_unsatisfiable_ranges() {
        let root = digits();
        for range in ["bytes=10-", "bytes=-0", "bytes=5-2"] {
            let r = get_with(&root, &[("range", range)]).await;
            assert_eq!(r.status(), StatusCode::RANGE_NOT_SATISFIABLE, "{range}");
            assert_eq!(r.headers()[header::CONTENT_RANGE], "bytes */10", "{range}");
        }
    }

    #[tokio::test]
    async fn ignores_multi_and_malformed_ranges() {
        let root = digits();
        for range in ["bytes=0-1,4-5", "items=0-1", "bytes=x-y"] {
            let r = get_with(&root, &[("range", range)]).await;
            assert_eq!(r.status(), StatusCode::OK, "{range}");
            assert_eq!(body(r).await.len(), 10, "{range}");
        }
    }

    #[tokio::test]
    async fn sends_validators_and_answers_304() {
        let root = digits();
        let r = get_with(&root, &[]).await;
        assert_eq!(r.status(), StatusCode::OK);
        let etag = r.headers()[header::ETAG].to_str().unwrap().to_string();
        let modified = r.headers()[header::LAST_MODIFIED]
            .to_str()
            .unwrap()
            .to_string();

        let r = get_with(&root, &[("if-none-match", &etag)]).await;
        assert_eq!(r.status(), StatusCode::NOT_MODIFIED);
        assert_eq!(r.headers()[header::ETAG], etag.as_str());
        assert_eq!(r.headers()[header::ACCEPT_RANGES], "bytes");
        assert!(body(r).await.is_empty());

        let r = get_with(&root, &[("if-none-match", "\"other\"")]).await;
        assert_eq!(r.status(), StatusCode::OK);

        let r = get_with(&root, &[("if-modified-since", &modified)]).await;
        assert_eq!(r.status(), StatusCode::NOT_MODIFIED);
    }

    #[tokio::test]
    async fn streams_large_files_whole_and_in_ranges() {
        use axum::body::HttpBody;
        let root = root();
        let data: Vec<u8> = (0..3_145_728usize).map(|i| (i % 251) as u8).collect();
        std::fs::write(root.path().join("demo/assets/big.png"), &data).unwrap();
        let fetch = |headers: Vec<(&'static str, &'static str)>| {
            let mut map = HeaderMap::new();
            for (k, v) in headers {
                map.insert(
                    axum::http::HeaderName::from_static(k),
                    HeaderValue::from_static(v),
                );
            }
            let path = root.path().to_path_buf();
            async move {
                asset_response(
                    path,
                    Arc::default(),
                    "demo".into(),
                    "assets/big.png".into(),
                    &map,
                )
                .await
            }
        };

        let r = fetch(vec![]).await;
        assert_eq!(r.status(), StatusCode::OK);
        assert_eq!(r.headers()[header::CONTENT_LENGTH], "3145728");
        // A streamed body has no exact size of its own; a buffered one does.
        assert!(r.body().size_hint().exact().is_none());
        assert_eq!(r.headers()[header::CONTENT_SECURITY_POLICY], ASSET_CSP);
        assert_eq!(r.headers()[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
        assert_eq!(body(r).await, data);

        let r = fetch(vec![("range", "bytes=1048576-")]).await;
        assert_eq!(r.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(r.headers()[header::CONTENT_LENGTH], "2097152");
        assert_eq!(
            r.headers()[header::CONTENT_RANGE],
            "bytes 1048576-3145727/3145728"
        );
        assert!(r.body().size_hint().exact().is_none());
        assert_eq!(r.headers()[header::CONTENT_SECURITY_POLICY], ASSET_CSP);
        assert_eq!(r.headers()[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
        assert_eq!(body(r).await, data[1_048_576..]);
    }

    /// `demo` and `other`, each with the same 3 MB asset.
    fn two_big_themes() -> (tempfile::TempDir, Vec<u8>) {
        let root = tempfile::tempdir().unwrap();
        let data: Vec<u8> = (0..3_145_728usize).map(|i| (i % 251) as u8).collect();
        for id in ["demo", "other"] {
            let dir = root.path().join(id).join("assets");
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("big.png"), &data).unwrap();
        }
        (root, data)
    }

    async fn rest(body: Body) -> Vec<u8> {
        use futures::StreamExt;
        let mut out = Vec::new();
        let mut stream = body.into_data_stream();
        while let Some(Ok(chunk)) = stream.next().await {
            out.extend_from_slice(&chunk);
        }
        out
    }

    #[tokio::test]
    async fn closing_a_theme_ends_its_streams_and_lets_go_of_the_file() {
        use futures::StreamExt;
        let (root, data) = two_big_themes();
        let streams: Arc<AssetStreams> = Arc::default();
        let demo = get_from(&root, &streams, "demo", "assets/big.png").await;
        let other = get_from(&root, &streams, "other", "assets/big.png").await;
        assert_eq!(demo.status(), StatusCode::OK);
        let mut demo = demo.into_body().into_data_stream();
        let mut other = other.into_body().into_data_stream();
        // One chunk each, then the client stops reading, like a paused
        // <video>: nothing polls these bodies while the theme is replaced.
        let mut demo_read = demo.next().await.unwrap().unwrap().to_vec();
        let mut other_read = other.next().await.unwrap().unwrap().to_vec();

        let s = streams.clone();
        let ended = tokio::task::spawn_blocking(move || s.close("demo").ended)
            .await
            .unwrap();
        assert!(ended, "every stream of demo closed its file");

        while let Some(Ok(chunk)) = demo.next().await {
            demo_read.extend_from_slice(&chunk);
        }
        assert!(demo_read.len() < data.len(), "demo's body ended early");
        assert_eq!(demo_read, data[..demo_read.len()]);

        while let Some(Ok(chunk)) = other.next().await {
            other_read.extend_from_slice(&chunk);
        }
        assert_eq!(other_read, data, "other's body is untouched");
    }

    #[tokio::test]
    async fn a_theme_being_replaced_serves_nothing_until_it_is_done() {
        let root = root();
        let streams: Arc<AssetStreams> = Arc::default();
        let closing = streams.close("demo");
        assert_eq!(
            get_from(&root, &streams, "demo", "assets/a.png")
                .await
                .status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
        drop(closing);
        let r = get_from(&root, &streams, "demo", "assets/a.png").await;
        assert_eq!(r.status(), StatusCode::OK);
        assert_eq!(rest(r.into_body()).await, b"png");
    }

    #[tokio::test]
    async fn not_modified_and_unsatisfiable_keep_the_hardening_headers() {
        let root = digits();
        let r = get_with(&root, &[]).await;
        let etag = r.headers()[header::ETAG].to_str().unwrap().to_string();
        for r in [
            get_with(&root, &[("if-none-match", &etag)]).await,
            get_with(&root, &[("range", "bytes=10-")]).await,
        ] {
            assert_eq!(r.headers()[header::CONTENT_SECURITY_POLICY], ASSET_CSP);
            assert_eq!(r.headers()[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
        }
    }

    #[tokio::test]
    async fn a_stale_if_range_sends_the_whole_file() {
        let root = digits();
        let r = get_with(&root, &[("range", "bytes=0-1"), ("if-range", "\"stale\"")]).await;
        assert_eq!(r.status(), StatusCode::OK);
        assert_eq!(body(r).await.len(), 10);
    }

    #[tokio::test]
    async fn serves_assets_with_hardening_headers() {
        let root = root();
        for (path, ct) in [
            ("assets/a.svg", "image/svg+xml"),
            ("assets/a.png", "image/png"),
        ] {
            let r = get(&root, "demo", path).await;
            assert_eq!(r.status(), StatusCode::OK);
            let h = r.headers();
            assert_eq!(h[header::CONTENT_TYPE], ct);
            assert_eq!(h[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
            assert_eq!(h[header::CONTENT_SECURITY_POLICY], ASSET_CSP);
            assert_eq!(h[header::CACHE_CONTROL], "no-cache");
        }
        assert_eq!(
            ASSET_CSP,
            "default-src 'none'; style-src 'unsafe-inline'; img-src 'self' data:"
        );
    }

    #[tokio::test]
    async fn maps_errors_to_statuses() {
        let root = root();
        assert_eq!(
            get(&root, "demo", "assets/missing.png").await.status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            get(&root, "nope", "assets/a.png").await.status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            get(&root, "demo", "../x.png").await.status(),
            StatusCode::BAD_REQUEST
        );
    }
}
