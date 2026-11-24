//! The manifest's `background`, `decorations`, `effects`, `layout` and
//! `sounds` keys.
//!
//! The core only checks values and paths. A `None` means "the renderer's
//! default"; the one default table lives in the renderer.

use std::path::Path;

use serde_json::{Map, Value};

use super::error::ThemeError;
use super::files::{FileKind, ThemeFiles, normalize_rel_path, shown};
use super::manifest::{check_path, normalize_color};

pub const DECORATION_ANCHORS: [&str; 14] = [
    "content",
    "nav",
    "nav.logo",
    "panel",
    "library.tile",
    "modal",
    "season-dialog",
    "section-title",
    "library.group",
    "library.toolbar",
    "instance.header",
    "server.header",
    "settings.nav",
    "page-title",
];

const FITS: [&str; 4] = ["cover", "contain", "fill", "none"];
const POSITIONS: [&str; 9] = [
    "center",
    "top",
    "bottom",
    "left",
    "right",
    "top left",
    "top right",
    "bottom left",
    "bottom right",
];
const EDGES: [&str; 8] = [
    "top",
    "bottom",
    "left",
    "right",
    "top-left",
    "top-right",
    "bottom-left",
    "bottom-right",
];
const PICKS: [&str; 3] = ["random", "cycle", "first"];
const REPEATS: [&str; 3] = ["none", "x", "y"];
const BULBS: [&str; 2] = ["teardrop", "round"];
const NAVS: [&str; 3] = ["standard", "compact", "rail"];
const TILE_STYLES: [&str; 2] = ["overlay", "caption"];
const PRESETS: [&str; 3] = ["fall", "float", "drift"];
const SOUND_KEYS: [&str; 5] = [
    "launch",
    "downloadComplete",
    "click",
    "notification",
    "error",
];

#[derive(Debug, Clone, PartialEq)]
pub enum Length {
    Px(f64),
    Percent(f64),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Background {
    pub src: String,
    pub video: bool,
    pub motion: Vec<String>,
    pub fit: Option<String>,
    pub position: Option<String>,
    pub dim: Option<f64>,
    pub blur: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Decoration {
    pub anchor: String,
    pub edge: String,
    pub x: Option<Length>,
    pub y: Option<Length>,
    pub width: Option<Length>,
    pub height: Option<Length>,
    pub images: Vec<String>,
    pub pick: Option<String>,
    pub repeat: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    Snow {
        density: Option<f64>,
        speed: Option<f64>,
        min_size: Option<f64>,
        max_size: Option<f64>,
        wind: Option<f64>,
        color: Option<String>,
        layers: Option<u8>,
    },
    StringLights {
        colors: Vec<String>,
        spacing: Option<f64>,
        droop: Option<f64>,
        size: Option<f64>,
        twinkle: Option<f64>,
        bulb: Option<String>,
    },
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

/// The layout menu: `None` means "the renderer's default".
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Layout {
    pub nav: Option<String>,
    pub panels: Option<bool>,
    pub tile_style: Option<String>,
    pub tile_size: Option<u8>,
}

/// Paths of the theme's own audio files, one per event.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Sounds {
    pub launch: Option<String>,
    pub download_complete: Option<String>,
    pub click: Option<String>,
    pub notification: Option<String>,
    pub error: Option<String>,
}

fn invalid<T>(msg: impl Into<String>) -> Result<T, ThemeError> {
    Err(ThemeError::Invalid(msg.into()))
}

pub(super) fn as_object<'a>(
    v: &'a Value,
    field: &str,
) -> Result<&'a Map<String, Value>, ThemeError> {
    match v.as_object() {
        Some(o) => Ok(o),
        None => invalid(format!("{field} must be an object")),
    }
}

pub(super) fn warn_unknown(
    o: &Map<String, Value>,
    known: &[&str],
    prefix: &str,
    noun: &str,
    warnings: &mut Vec<String>,
) {
    for key in o.keys() {
        if !known.contains(&key.as_str()) {
            warnings.push(format!(
                "{prefix}.{} isn't a {noun} key and was ignored",
                shown(key)
            ));
        }
    }
}

fn opt_num(
    o: &Map<String, Value>,
    key: &str,
    prefix: &str,
    min: f64,
    max: f64,
) -> Result<Option<f64>, ThemeError> {
    match o.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => match v.as_f64() {
            Some(n) if n.is_finite() && (min..=max).contains(&n) => Ok(Some(n)),
            _ => invalid(format!(
                "{prefix}.{key} must be a number from {min} to {max}"
            )),
        },
    }
}

fn opt_choice(
    o: &Map<String, Value>,
    key: &str,
    prefix: &str,
    allowed: &[&str],
) -> Result<Option<String>, ThemeError> {
    match o.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) if allowed.contains(&s.as_str()) => Ok(Some(s.clone())),
        _ => invalid(format!(
            "{prefix}.{key} must be one of: {}",
            allowed.join(", ")
        )),
    }
}

fn opt_color(
    o: &Map<String, Value>,
    key: &str,
    prefix: &str,
) -> Result<Option<String>, ThemeError> {
    match o.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => match v.as_str().and_then(normalize_color) {
            Some(c) => Ok(Some(c)),
            None => invalid(format!(
                "{prefix}.{key} must be three numbers from 0 to 255, like \"255 255 255\""
            )),
        },
    }
}

fn path_array(
    o: &Map<String, Value>,
    key: &str,
    prefix: &str,
    max: usize,
    min: usize,
    files: &ThemeFiles,
) -> Result<Vec<String>, ThemeError> {
    let field = format!("{prefix}.{key}");
    let items: &[Value] = match o.get(key) {
        None | Some(Value::Null) if min == 0 => &[],
        Some(Value::Array(a)) => a.as_slice(),
        _ => return invalid(format!("{field} must be a list of {min} to {max} images")),
    };
    if items.len() < min || items.len() > max {
        return invalid(format!("{field} must have {min} to {max} images"));
    }
    let mut out = Vec::new();
    for (j, item) in items.iter().enumerate() {
        let Some(raw) = item.as_str() else {
            return invalid(format!("{field}[{j}] must be a path"));
        };
        out.push(check_path(
            raw,
            &format!("{field}[{j}]"),
            FileKind::Image,
            "an image",
            files,
        )?);
    }
    Ok(out)
}

pub fn parse_background(
    v: &Value,
    files: &ThemeFiles,
    warnings: &mut Vec<String>,
) -> Result<Background, ThemeError> {
    let o = as_object(v, "background")?;
    warn_unknown(
        o,
        &["src", "motion", "fit", "position", "dim", "blur"],
        "background",
        "background",
        warnings,
    );
    let Some(raw) = o.get("src").and_then(Value::as_str) else {
        return invalid("background.src is required and must be a path");
    };
    let video = normalize_rel_path(raw)
        .and_then(|rel| files.kind_of(&rel))
        .is_some_and(|k| k == FileKind::Video);
    let src = if video {
        check_path(raw, "background.src", FileKind::Video, "a video", files)?
    } else {
        check_path(
            raw,
            "background.src",
            FileKind::Image,
            "an image or a video",
            files,
        )?
    };
    let motion = path_array(o, "motion", "background", 3, 0, files)?;
    Ok(Background {
        src,
        video,
        motion,
        fit: opt_choice(o, "fit", "background", &FITS)?,
        position: opt_choice(o, "position", "background", &POSITIONS)?,
        dim: opt_num(o, "dim", "background", 0.0, 0.9)?,
        blur: opt_num(o, "blur", "background", 0.0, 40.0)?,
    })
}

fn parse_length(
    o: &Map<String, Value>,
    key: &str,
    prefix: &str,
) -> Result<Option<Length>, ThemeError> {
    let field = format!("{prefix}.{key}");
    let bad = || {
        invalid(format!(
            "{field} must be a number of px (up to 4000) or a percentage like \"12.5%\""
        ))
    };
    match o.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => match n.as_f64() {
            Some(n) if n.is_finite() && n.abs() <= 4000.0 => Ok(Some(Length::Px(n))),
            _ => bad(),
        },
        Some(Value::String(s)) => {
            let Some(num) = s.strip_suffix('%') else {
                return bad();
            };
            let digits = num.strip_prefix('-').unwrap_or(num);
            let (int, frac) = match digits.split_once('.') {
                Some((i, f)) => (i, Some(f)),
                None => (digits, None),
            };
            let all_digits = |t: &str, max: usize| {
                (1..=max).contains(&t.len()) && t.bytes().all(|b| b.is_ascii_digit())
            };
            if !all_digits(int, 4) || frac.is_some_and(|f| !all_digits(f, 3)) {
                return bad();
            }
            match num.parse::<f64>() {
                Ok(n) if n.abs() <= 1000.0 => Ok(Some(Length::Percent(n))),
                _ => bad(),
            }
        }
        Some(_) => bad(),
    }
}

pub fn parse_decorations(
    v: &Value,
    files: &ThemeFiles,
    warnings: &mut Vec<String>,
) -> Result<Vec<Decoration>, ThemeError> {
    let Some(items) = v.as_array() else {
        return invalid("decorations must be a list of at most 32 entries");
    };
    if items.len() > 32 {
        return invalid("decorations can have at most 32 entries");
    }
    let mut out = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let prefix = format!("decorations[{i}]");
        let o = as_object(item, &prefix)?;
        warn_unknown(
            o,
            &[
                "anchor", "edge", "x", "y", "width", "height", "images", "pick", "repeat",
            ],
            &prefix,
            "decoration",
            warnings,
        );
        let Some(anchor) = o.get("anchor").and_then(Value::as_str) else {
            return invalid(format!("{prefix}.anchor is required"));
        };
        if !DECORATION_ANCHORS.contains(&anchor) {
            warnings.push(format!(
                "{prefix}.anchor \"{}\" can't be decorated and was ignored",
                super::files::shown(anchor)
            ));
            continue;
        }
        let Some(edge) = opt_choice(o, "edge", &prefix, &EDGES)? else {
            return invalid(format!("{prefix}.edge is required"));
        };
        let x = parse_length(o, "x", &prefix)?;
        let y = parse_length(o, "y", &prefix)?;
        let width = parse_length(o, "width", &prefix)?;
        let height = parse_length(o, "height", &prefix)?;
        for (key, v) in [("width", &width), ("height", &height)] {
            if matches!(v, Some(Length::Px(n) | Length::Percent(n)) if *n <= 0.0) {
                return invalid(format!("{prefix}.{key} must be larger than 0"));
            }
        }
        let needs_height = matches!(edge.as_str(), "top" | "bottom") || edge.contains('-');
        let needs_width = matches!(edge.as_str(), "left" | "right") || edge.contains('-');
        if needs_height && height.is_none() {
            return invalid(format!(
                "{prefix}.height is required for the \"{edge}\" edge"
            ));
        }
        if needs_width && width.is_none() {
            return invalid(format!(
                "{prefix}.width is required for the \"{edge}\" edge"
            ));
        }
        let images = path_array(o, "images", &prefix, 64, 1, files)?;
        out.push(Decoration {
            anchor: anchor.to_string(),
            edge,
            x,
            y,
            width,
            height,
            images,
            pick: opt_choice(o, "pick", &prefix, &PICKS)?,
            repeat: opt_choice(o, "repeat", &prefix, &REPEATS)?,
        });
    }
    Ok(out)
}

pub fn parse_layout(v: &Value, warnings: &mut Vec<String>) -> Result<Layout, ThemeError> {
    let o = as_object(v, "layout")?;
    warn_unknown(
        o,
        &["nav", "panels", "tileStyle", "tileSize"],
        "layout",
        "layout",
        warnings,
    );
    let panels = match o.get("panels") {
        None | Some(Value::Null) => None,
        Some(Value::Bool(b)) => Some(*b),
        _ => return invalid("layout.panels must be true or false"),
    };
    let tile_size = match o.get("tileSize") {
        None | Some(Value::Null) => None,
        Some(n) => match n.as_u64() {
            Some(n @ 1..=5) => Some(n as u8),
            _ => return invalid("layout.tileSize must be a whole number from 1 to 5"),
        },
    };
    Ok(Layout {
        nav: opt_choice(o, "nav", "layout", &NAVS)?,
        panels,
        tile_style: opt_choice(o, "tileStyle", "layout", &TILE_STYLES)?,
        tile_size,
    })
}

pub fn parse_sounds(
    v: &Value,
    files: &ThemeFiles,
    warnings: &mut Vec<String>,
) -> Result<Sounds, ThemeError> {
    let o = as_object(v, "sounds")?;
    warn_unknown(o, &SOUND_KEYS, "sounds", "sound", warnings);
    let sound = |key: &str| -> Result<Option<String>, ThemeError> {
        let field = format!("sounds.{key}");
        match o.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(raw)) => Ok(Some(check_path(
                raw,
                &field,
                FileKind::Audio,
                "an audio file",
                files,
            )?)),
            Some(_) => invalid(format!("{field} must be a path")),
        }
    };
    Ok(Sounds {
        launch: sound("launch")?,
        download_complete: sound("downloadComplete")?,
        click: sound("click")?,
        notification: sound("notification")?,
        error: sound("error")?,
    })
}

pub fn parse_effects(
    v: &Value,
    files: &ThemeFiles,
    warnings: &mut Vec<String>,
) -> Result<Vec<Effect>, ThemeError> {
    let Some(items) = v.as_array() else {
        return invalid("effects must be a list of at most 4 entries");
    };
    if items.len() > 4 {
        return invalid("effects can have at most 4 entries");
    }
    let mut out: Vec<Effect> = Vec::new();
    let mut seen: Vec<&str> = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let prefix = format!("effects[{i}]");
        let o = as_object(item, &prefix)?;
        let Some(kind) = o.get("type").and_then(Value::as_str) else {
            return invalid(format!("{prefix}.type is required"));
        };
        if !matches!(kind, "snow" | "stringLights" | "particles") {
            warnings.push(format!(
                "{prefix}: \"{}\" isn't supported by this version and was ignored",
                shown(kind)
            ));
            continue;
        }
        if seen.contains(&kind) {
            warnings.push(format!(
                "{prefix}: a second \"{}\" effect was ignored",
                shown(kind)
            ));
            continue;
        }
        seen.push(kind);
        if kind == "particles" {
            warn_unknown(
                o,
                &[
                    "type", "images", "preset", "density", "speed", "minSize", "maxSize", "rotate",
                    "opacity",
                ],
                &prefix,
                "particles",
                warnings,
            );
            let images = path_array(o, "images", &prefix, 8, 1, files)?;
            let Some(preset) = opt_choice(o, "preset", &prefix, &PRESETS)? else {
                return invalid(format!(
                    "{prefix}.preset is required and must be one of: {}",
                    PRESETS.join(", ")
                ));
            };
            let min_size = opt_num(o, "minSize", &prefix, 4.0, 96.0)?;
            let max_size = opt_num(o, "maxSize", &prefix, 4.0, 96.0)?;
            if let (Some(a), Some(b)) = (min_size, max_size) {
                if a > b {
                    return invalid(format!("{prefix}.minSize can't be larger than maxSize"));
                }
            }
            let rotate = match o.get("rotate") {
                None | Some(Value::Null) => None,
                Some(Value::Bool(b)) => Some(*b),
                _ => return invalid(format!("{prefix}.rotate must be true or false")),
            };
            out.push(Effect::Particles {
                images,
                preset,
                density: opt_num(o, "density", &prefix, 0.0, 30.0)?,
                speed: opt_num(o, "speed", &prefix, 0.2, 3.0)?,
                min_size,
                max_size,
                rotate,
                opacity: opt_num(o, "opacity", &prefix, 0.1, 1.0)?,
            });
        } else if kind == "snow" {
            warn_unknown(
                o,
                &[
                    "type", "density", "speed", "minSize", "maxSize", "wind", "color", "layers",
                ],
                &prefix,
                "snow",
                warnings,
            );
            let min_size = opt_num(o, "minSize", &prefix, 0.3, 8.0)?;
            let max_size = opt_num(o, "maxSize", &prefix, 0.3, 8.0)?;
            if let (Some(a), Some(b)) = (min_size, max_size) {
                if a > b {
                    return invalid(format!("{prefix}.minSize can't be larger than maxSize"));
                }
            }
            let layers = match o.get("layers") {
                None | Some(Value::Null) => None,
                Some(l) => match l.as_u64() {
                    Some(n @ 1..=3) => Some(n as u8),
                    _ => return invalid(format!("{prefix}.layers must be 1, 2 or 3")),
                },
            };
            out.push(Effect::Snow {
                density: opt_num(o, "density", &prefix, 0.0, 60.0)?,
                speed: opt_num(o, "speed", &prefix, 0.2, 3.0)?,
                min_size,
                max_size,
                wind: opt_num(o, "wind", &prefix, -60.0, 60.0)?,
                color: opt_color(o, "color", &prefix)?,
                layers,
            });
        } else {
            warn_unknown(
                o,
                &[
                    "type", "colors", "spacing", "droop", "size", "twinkle", "bulb",
                ],
                &prefix,
                "stringLights",
                warnings,
            );
            let colors = match o.get("colors") {
                None | Some(Value::Null) => Vec::new(),
                Some(Value::Array(a)) if (1..=12).contains(&a.len()) => {
                    let mut cs = Vec::new();
                    for (j, c) in a.iter().enumerate() {
                        match c.as_str().and_then(normalize_color) {
                            Some(c) => cs.push(c),
                            None => {
                                return invalid(format!(
                                    "{prefix}.colors[{j}] must be three numbers from 0 to 255, like \"255 200 80\""
                                ));
                            }
                        }
                    }
                    cs
                }
                _ => return invalid(format!("{prefix}.colors must have 1 to 12 colors")),
            };
            out.push(Effect::StringLights {
                colors,
                spacing: opt_num(o, "spacing", &prefix, 60.0, 400.0)?,
                droop: opt_num(o, "droop", &prefix, 4.0, 40.0)?,
                size: opt_num(o, "size", &prefix, 0.5, 2.0)?,
                twinkle: opt_num(o, "twinkle", &prefix, 0.0, 1.0)?,
                bulb: opt_choice(o, "bulb", &prefix, &BULBS)?,
            });
        }
    }
    Ok(out)
}

/// An SVG counts as animated when its text has SMIL animation elements
/// (`<animate`, which also matches `<animateTransform` and `<animateMotion`,
/// or `<set`) or CSS `@keyframes` (including `@-webkit-keyframes`). Reduced
/// motion can't pause an animated SVG that is drawn as an image.
pub fn svg_is_animated(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    if lower.contains("@keyframes") || lower.contains("@-webkit-keyframes") {
        return true;
    }
    if lower.contains("<animate") {
        return true;
    }
    lower.match_indices("<set").any(|(at, _)| {
        lower[at + 4..]
            .chars()
            .next()
            .is_none_or(|c| c.is_whitespace() || c == '/' || c == '>')
    })
}

/// A GIF is animated with a NETSCAPE2.0 block or more than one image.
fn gif_is_animated(b: &[u8]) -> bool {
    if !(b.starts_with(b"GIF87a") || b.starts_with(b"GIF89a")) || b.len() < 13 {
        return false;
    }
    if b.windows(11).any(|w| w == b"NETSCAPE2.0") {
        return true;
    }
    let mut at = 13;
    if b[10] & 0x80 != 0 {
        at += 3 * (1usize << ((b[10] & 7) + 1));
    }
    let skip_sub_blocks = |mut at: usize| {
        while at < b.len() && b[at] != 0 {
            at += b[at] as usize + 1;
        }
        at + 1
    };
    let mut images = 0;
    while at < b.len() {
        match b[at] {
            0x2C => {
                images += 1;
                if images > 1 {
                    return true;
                }
                if at + 10 > b.len() {
                    return false;
                }
                let packed = b[at + 9];
                at += 10;
                if packed & 0x80 != 0 {
                    at += 3 * (1usize << ((packed & 7) + 1));
                }
                at += 1; // LZW minimum code size
                at = skip_sub_blocks(at);
            }
            0x21 => at = skip_sub_blocks(at + 2),
            _ => return false,
        }
    }
    false
}

/// A WebP is animated with the VP8X animation flag or an ANIM chunk.
fn webp_is_animated(b: &[u8]) -> bool {
    if b.len() < 12 || &b[0..4] != b"RIFF" || &b[8..12] != b"WEBP" {
        return false;
    }
    let mut at = 12;
    while at + 8 <= b.len() {
        let id = &b[at..at + 4];
        if id == b"ANIM" {
            return true;
        }
        if id == b"VP8X" && b.get(at + 8).is_some_and(|f| f & 0x02 != 0) {
            return true;
        }
        let size = u32::from_le_bytes([b[at + 4], b[at + 5], b[at + 6], b[at + 7]]) as usize;
        at = at.saturating_add(8 + size + (size & 1));
    }
    false
}

/// An APNG has an `acTL` chunk before the first `IDAT`.
fn png_is_animated(b: &[u8]) -> bool {
    if !b.starts_with(b"\x89PNG\r\n\x1a\n") {
        return false;
    }
    let mut at = 8;
    while at + 8 <= b.len() {
        let id = &b[at + 4..at + 8];
        if id == b"acTL" {
            return true;
        }
        if id == b"IDAT" {
            return false;
        }
        let size = u32::from_be_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]]) as usize;
        at = at.saturating_add(12 + size);
    }
    false
}

/// Warnings for animated SVGs, GIFs, WebPs and APNGs used as the background
/// or as decoration images. Unreadable files are skipped; only the first 2 MB
/// of each is looked at.
pub fn animated_image_warnings(
    dir: &Path,
    background: Option<&Background>,
    decorations: &[Decoration],
) -> Vec<String> {
    use std::io::Read;

    let mut checked: Vec<(String, String)> = Vec::new();
    if let Some(b) = background {
        checked.push(("background.src".to_string(), b.src.clone()));
    }
    for (i, d) in decorations.iter().enumerate() {
        for (j, img) in d.images.iter().enumerate() {
            checked.push((format!("decorations[{i}].images[{j}]"), img.clone()));
        }
    }
    let mut seen: Vec<String> = Vec::new();
    let mut out = Vec::new();
    for (field, rel) in checked {
        let ext = rel.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
        if !matches!(ext.as_str(), "svg" | "gif" | "webp" | "png") || seen.contains(&rel) {
            continue;
        }
        seen.push(rel.clone());
        let Ok(file) = std::fs::File::open(dir.join(&rel)) else {
            continue;
        };
        let mut bytes = Vec::new();
        if file.take(2 * 1024 * 1024).read_to_end(&mut bytes).is_err() {
            continue;
        }
        let (animated, what) = match ext.as_str() {
            "svg" => (svg_is_animated(&String::from_utf8_lossy(&bytes)), "SVG"),
            "gif" => (gif_is_animated(&bytes), "GIF"),
            "webp" => (webp_is_animated(&bytes), "WebP"),
            _ => (png_is_animated(&bytes), "PNG"),
        };
        if animated {
            out.push(format!(
                "{field} \"{}\" is an animated {what}, and reduced motion can't pause animated images; put the animation in background.motion layers instead",
                super::files::shown(&rel)
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn files(list: &[&str]) -> ThemeFiles {
        ThemeFiles {
            files: list.iter().map(|f| (f.to_string(), 10)).collect(),
            fingerprint: 0,
        }
    }

    const ALL: [&str; 6] = [
        "assets/scene.svg",
        "assets/smoke.svg",
        "assets/loop.webm",
        "assets/a.svg",
        "assets/b.svg",
        "theme.css",
    ];

    fn msg<T: std::fmt::Debug>(r: Result<T, ThemeError>) -> String {
        match r {
            Err(ThemeError::Invalid(m)) => m,
            other => panic!("expected Invalid, got {other:?}"),
        }
    }

    fn bg(v: Value) -> Result<Background, ThemeError> {
        parse_background(&v, &files(&ALL), &mut Vec::new())
    }

    fn deco(v: Value) -> Result<Vec<Decoration>, ThemeError> {
        parse_decorations(&v, &files(&ALL), &mut Vec::new())
    }

    /// The renderer keeps its own copy of the anchor list (it renders the
    /// slots); both must name the same anchors in the same order.
    #[test]
    fn the_anchor_list_matches_the_renderer() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../apps/desktop/packages/mainWindow/src/themes/decorations.ts");
        let text = std::fs::read_to_string(&path).unwrap();
        let start = text
            .find("export const DECORATION_ANCHORS = [")
            .expect("DECORATION_ANCHORS in decorations.ts");
        let rest = &text[start..];
        let list = &rest[rest.find('[').unwrap() + 1..rest.find(']').unwrap()];
        let renderer: Vec<&str> = list
            .split(',')
            .map(|s| s.trim().trim_matches('"'))
            .filter(|s| !s.is_empty())
            .collect();
        assert_eq!(renderer, DECORATION_ANCHORS.to_vec());
    }

    fn fx(v: Value) -> Result<Vec<Effect>, ThemeError> {
        parse_effects(&v, &files(&ALL), &mut Vec::new())
    }

    #[test]
    fn parses_background_decorations_and_effects() {
        let mut w = Vec::new();
        let b = parse_background(
            &json!({"src": "assets/scene.svg", "motion": ["assets/smoke.svg"], "fit": "cover",
                    "position": "bottom right", "dim": 0.2, "blur": 4}),
            &files(&ALL),
            &mut w,
        )
        .unwrap();
        assert_eq!(
            b,
            Background {
                src: "assets/scene.svg".into(),
                video: false,
                motion: vec!["assets/smoke.svg".into()],
                fit: Some("cover".into()),
                position: Some("bottom right".into()),
                dim: Some(0.2),
                blur: Some(4.0),
            }
        );
        let d = parse_decorations(
            &json!([{"anchor": "library.tile", "edge": "top-right", "x": -30, "y": "-16.3%",
                     "width": 120, "height": "50%", "images": ["./assets/a.svg", "assets/b.svg"],
                     "pick": "random", "repeat": "none"}]),
            &files(&ALL),
            &mut w,
        )
        .unwrap();
        assert_eq!(d[0].x, Some(Length::Px(-30.0)));
        assert_eq!(d[0].y, Some(Length::Percent(-16.3)));
        assert_eq!(d[0].height, Some(Length::Percent(50.0)));
        assert_eq!(d[0].images, vec!["assets/a.svg", "assets/b.svg"]);
        assert_eq!(d[0].pick.as_deref(), Some("random"));
        let e = parse_effects(
            &json!([{"type": "snow", "density": 20, "minSize": 1, "maxSize": 3, "color": "255  255 255", "layers": 2},
                    {"type": "stringLights", "colors": ["255 200 80", "80 200 255"], "spacing": 120, "bulb": "teardrop"}]),
            &files(&ALL),
            &mut w,
        )
        .unwrap();
        assert_eq!(e.len(), 2);
        assert_eq!(
            e[0],
            Effect::Snow {
                density: Some(20.0),
                speed: None,
                min_size: Some(1.0),
                max_size: Some(3.0),
                wind: None,
                color: Some("255 255 255".into()),
                layers: Some(2),
            }
        );
        assert!(matches!(&e[1], Effect::StringLights { colors, .. } if colors.len() == 2));
        assert!(w.is_empty(), "{w:?}");
    }

    #[test]
    fn a_video_background_is_marked() {
        let b = bg(json!({"src": "assets/loop.webm"})).unwrap();
        assert!(b.video);
        let m = msg(bg(
            json!({"src": "assets/scene.svg", "motion": ["assets/loop.webm"]}),
        ));
        assert!(m.contains("background.motion"), "{m}");
    }

    #[test]
    fn refuses_bad_extras() {
        let src = json!("assets/scene.svg");
        for (key, val) in [
            ("fit", json!("tile")),
            ("position", json!("middle")),
            ("dim", json!(1)),
            ("blur", json!(41)),
        ] {
            let m = msg(bg(json!({"src": src, key: val})));
            assert!(m.contains(&format!("background.{key}")), "{m}");
        }
        let d = |extra: Value| {
            let mut o =
                json!({"anchor": "panel", "edge": "left", "width": 10, "images": ["assets/a.svg"]});
            for (k, v) in extra.as_object().unwrap() {
                o[k] = v.clone();
            }
            deco(json!([o]))
        };
        assert!(msg(d(json!({"edge": "diagonal"}))).contains("decorations[0].edge"));
        assert!(msg(d(json!({"edge": "top"}))).contains("decorations[0].height"));
        assert!(msg(d(json!({"images": []}))).contains("decorations[0].images"));
        let many: Vec<&str> = vec!["assets/a.svg"; 65];
        assert!(msg(d(json!({"images": many}))).contains("decorations[0].images"));
        assert!(msg(d(json!({"x": "10px"}))).contains("decorations[0].x"));
        assert!(msg(d(json!({"x": "12%x"}))).contains("decorations[0].x"));
        assert!(msg(d(json!({"x": 4001}))).contains("decorations[0].x"));
        assert!(msg(d(json!({"x": "1001%"}))).contains("decorations[0].x"));
        assert!(d(json!({"x": "-12.345%"})).is_ok());
        assert!(msg(fx(json!([{"type": "snow", "density": 61}]))).contains("effects[0].density"));
        assert!(
            msg(fx(json!([{"type": "snow", "minSize": 3, "maxSize": 2}])))
                .contains("effects[0].minSize")
        );
        assert!(
            msg(fx(json!([{"type": "stringLights", "colors": ["#fff"]}])))
                .contains("effects[0].colors")
        );
        assert!(msg(fx(json!({}))).contains("effects"));
    }

    #[test]
    fn the_page_level_hooks_can_be_decorated() {
        let mut w = Vec::new();
        let d = parse_decorations(
            &json!([
                {"anchor": "section-title", "edge": "top", "height": 5, "images": ["assets/a.svg"]},
                {"anchor": "instance.header", "edge": "top", "height": 5, "images": ["assets/a.svg"]},
                {"anchor": "page-title", "edge": "top", "height": 5, "images": ["assets/a.svg"]},
                {"anchor": "ad-column-background", "edge": "top", "height": 5, "images": ["assets/a.svg"]},
                {"anchor": "button", "edge": "top", "height": 5, "images": ["assets/a.svg"]}
            ]),
            &files(&ALL),
            &mut w,
        )
        .unwrap();
        assert_eq!(d.len(), 3);
        assert_eq!(w.len(), 2, "{w:?}");
        assert!(w.iter().any(|m| m.contains("ad-column-background")));
        assert!(w.iter().any(|m| m.contains("\"button\"")));
    }

    #[test]
    fn unknown_anchors_and_effect_types_warn() {
        let mut w = Vec::new();
        let d = parse_decorations(
            &json!([
                {"anchor": "ad-column-background", "edge": "top", "height": 5, "images": ["assets/a.svg"]},
                {"anchor": "button", "edge": "top", "height": 5, "images": ["assets/a.svg"]},
                {"anchor": "nav", "edge": "top", "height": 5, "images": ["assets/a.svg"], "glow": 1}
            ]),
            &files(&ALL),
            &mut w,
        )
        .unwrap();
        assert_eq!(d.len(), 1);
        assert!(
            w.iter().any(|m| m.contains("ad-column-background")),
            "{w:?}"
        );
        assert!(w.iter().any(|m| m.contains("\"button\"")), "{w:?}");
        assert!(w.iter().any(|m| m.contains("glow")), "{w:?}");
        let mut w = Vec::new();
        let e = parse_effects(&json!([{"type": "confetti"}]), &files(&ALL), &mut w).unwrap();
        assert!(e.is_empty());
        assert_eq!(
            w,
            vec!["effects[0]: \"confetti\" isn't supported by this version and was ignored"]
        );
    }

    #[test]
    fn a_second_effect_of_a_type_is_ignored() {
        let mut w = Vec::new();
        let e = parse_effects(
            &json!([{"type": "snow", "density": 5}, {"type": "snow", "density": 9}]),
            &files(&ALL),
            &mut w,
        )
        .unwrap();
        assert_eq!(e.len(), 1);
        assert!(matches!(&e[0], Effect::Snow { density: Some(d), .. } if *d == 5.0));
        assert_eq!(w.len(), 1);
    }

    #[test]
    fn extras_paths_must_be_theme_assets() {
        for p in ["../x.svg", "assets/missing.svg", "theme.css"] {
            assert!(bg(json!({"src": p})).is_err(), "{p}");
            assert!(
                bg(json!({"src": "assets/scene.svg", "motion": [p]})).is_err(),
                "{p}"
            );
            let r = deco(json!([{"anchor": "nav", "edge": "top", "height": 5, "images": [p]}]));
            assert!(msg(r).contains("decorations[0].images[0]"), "{p}");
        }
    }

    #[test]
    fn animated_svgs_are_detected() {
        assert!(svg_is_animated(
            "<svg><animateTransform attributeName='transform'/></svg>"
        ));
        assert!(svg_is_animated("<svg><ANIMATE x='1'/></svg>"));
        assert!(svg_is_animated(
            "<svg><set attributeName='x' to='1'/></svg>"
        ));
        assert!(svg_is_animated("<svg><style>@keyframes a{}</style></svg>"));
        assert!(!svg_is_animated("<svg><setup/><rect/></svg>"));
        assert!(!svg_is_animated("<svg><rect/></svg>"));
        assert!(svg_is_animated(
            "<svg><style>@-webkit-keyframes a{}</style></svg>"
        ));
    }

    #[test]
    fn warns_about_animated_svgs_used_as_images() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("assets")).unwrap();
        std::fs::write(dir.path().join("assets/scene.svg"), "<svg><animate/></svg>").unwrap();
        std::fs::write(dir.path().join("assets/a.svg"), "<svg><rect/></svg>").unwrap();
        std::fs::write(dir.path().join("assets/b.svg"), "<svg><set to='1'/></svg>").unwrap();
        let b = bg(json!({"src": "assets/scene.svg"})).unwrap();
        let d = deco(json!([{"anchor": "nav", "edge": "top", "height": 5, "images": ["assets/a.svg", "assets/b.svg"]}])).unwrap();
        let w = animated_image_warnings(dir.path(), Some(&b), &d);
        assert_eq!(w.len(), 2, "{w:?}");
        assert!(w[0].contains("background.src") && w[0].contains("background.motion"));
        assert!(w[1].contains("decorations[0].images[1]"));
    }

    #[test]
    fn a_second_effect_has_its_own_warning() {
        let mut w = Vec::new();
        parse_effects(
            &json!([{"type": "snow"}, {"type": "snow"}]),
            &files(&ALL),
            &mut w,
        )
        .unwrap();
        assert_eq!(w, vec!["effects[1]: a second \"snow\" effect was ignored"]);
    }

    #[test]
    fn decoration_sizes_must_be_positive() {
        for (k, v) in [
            ("width", json!(0)),
            ("width", json!("0%")),
            ("height", json!(-5)),
        ] {
            let mut o = json!({"anchor": "nav", "edge": "top-left", "width": 10, "height": 10, "images": ["assets/a.svg"]});
            o[k] = v;
            assert!(msg(deco(json!([o]))).contains(&format!("decorations[0].{k}")));
        }
    }

    #[test]
    fn detects_animated_rasters() {
        let gif_head: &[u8] = b"GIF89a\x01\x00\x01\x00\x00\x00\x00";
        let image: &[u8] = b"\x2C\0\0\0\0\x01\0\x01\0\0\x02\x01\x00\x00";
        let mut one = gif_head.to_vec();
        one.extend_from_slice(image);
        one.push(b';');
        assert!(!gif_is_animated(&one));
        let mut two = gif_head.to_vec();
        two.extend_from_slice(image);
        two.extend_from_slice(image);
        assert!(gif_is_animated(&two));
        let mut net = gif_head.to_vec();
        net.extend_from_slice(b"\x21\xFF\x0BNETSCAPE2.0\x03\x01\0\0\0");
        assert!(gif_is_animated(&net));

        let mut webp = b"RIFF\x16\0\0\0WEBPVP8X\x0A\0\0\0".to_vec();
        webp.extend_from_slice(&[0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert!(webp_is_animated(&webp));
        webp[20] = 0;
        assert!(!webp_is_animated(&webp));
        let mut anim = b"RIFF\x10\0\0\0WEBPANIM\x04\0\0\0".to_vec();
        anim.extend_from_slice(&[0; 4]);
        assert!(webp_is_animated(&anim));

        let sig = b"\x89PNG\r\n\x1a\n";
        let mut apng = sig.to_vec();
        apng.extend_from_slice(b"\0\0\0\x08acTL\0\0\0\x02\0\0\0\0\0\0\0\0\0\0\0\0IDAT");
        assert!(png_is_animated(&apng));
        let mut plain = sig.to_vec();
        plain.extend_from_slice(b"\0\0\0\0IDAT\0\0\0\0\0\0\0\0acTL");
        assert!(!png_is_animated(&plain));
    }

    #[test]
    fn warns_about_animated_rasters() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("assets")).unwrap();
        let mut two = b"GIF89a\x01\x00\x01\x00\x00\x00\x00".to_vec();
        for _ in 0..2 {
            two.extend_from_slice(b"\x2C\0\0\0\0\x01\0\x01\0\0\x02\x01\x00\x00");
        }
        std::fs::write(dir.path().join("assets/loop.gif"), two).unwrap();
        let b = Background {
            src: "assets/loop.gif".into(),
            video: false,
            motion: vec![],
            fit: None,
            position: None,
            dim: None,
            blur: None,
        };
        let w = animated_image_warnings(dir.path(), Some(&b), &[]);
        assert_eq!(w.len(), 1, "{w:?}");
        assert!(w[0].contains("GIF") && w[0].contains("background.motion"));
    }

    const MEDIA: [&str; 9] = [
        "assets/a.ogg",
        "assets/b.mp3",
        "assets/c.wav",
        "assets/p1.png",
        "assets/p2.svg",
        "assets/pic.png",
        "assets/s1.wav",
        "assets/s2.wav",
        "theme.css",
    ];

    fn sounds(v: Value) -> Result<Sounds, ThemeError> {
        parse_sounds(&v, &files(&MEDIA), &mut Vec::new())
    }

    fn layout(v: Value) -> Result<Layout, ThemeError> {
        parse_layout(&v, &mut Vec::new())
    }

    fn particles(extra: Value) -> Result<Vec<Effect>, ThemeError> {
        let mut o = json!({"type": "particles", "images": ["assets/p1.png"], "preset": "fall"});
        for (k, v) in extra.as_object().unwrap() {
            if v.is_null() {
                o.as_object_mut().unwrap().remove(k);
            } else {
                o[k] = v.clone();
            }
        }
        parse_effects(&json!([o]), &files(&MEDIA), &mut Vec::new())
    }

    #[test]
    fn parses_layout_and_sounds() {
        let l =
            layout(json!({"nav": "rail", "panels": false, "tileStyle": "caption", "tileSize": 4}))
                .unwrap();
        assert_eq!(
            l,
            Layout {
                nav: Some("rail".into()),
                panels: Some(false),
                tile_style: Some("caption".into()),
                tile_size: Some(4),
            }
        );
        assert_eq!(layout(json!({})).unwrap(), Layout::default());
        let s = sounds(
            json!({"launch": "assets/a.ogg", "downloadComplete": "./assets/b.mp3",
            "click": "assets/c.wav", "notification": "assets/s1.wav", "error": "assets/s2.wav"}),
        )
        .unwrap();
        assert_eq!(
            s,
            Sounds {
                launch: Some("assets/a.ogg".into()),
                download_complete: Some("assets/b.mp3".into()),
                click: Some("assets/c.wav".into()),
                notification: Some("assets/s1.wav".into()),
                error: Some("assets/s2.wav".into()),
            }
        );
    }

    #[test]
    fn refuses_bad_layout_and_sounds() {
        for (key, val) in [
            ("nav", json!("side")),
            ("panels", json!("yes")),
            ("tileStyle", json!("grid")),
            ("tileSize", json!(0)),
            ("tileSize", json!(6)),
            ("tileSize", json!(2.5)),
        ] {
            let m = msg(layout(json!({key: val})));
            assert!(m.contains(&format!("layout.{key}")), "{m}");
        }
        assert!(msg(layout(json!([]))).contains("layout"));
        let m = msg(sounds(json!({"launch": "assets/pic.png"})));
        assert!(
            m.contains("sounds.launch") && m.contains("an audio file"),
            "{m}"
        );
        let m = msg(sounds(json!({"click": "assets/missing.ogg"})));
        assert!(m.contains("sounds.click"), "{m}");
        let m = msg(sounds(json!([])));
        assert!(m.contains("sounds"), "{m}");
        let m = msg(sounds(json!({"error": 5})));
        assert!(m.contains("sounds.error"), "{m}");
    }

    #[test]
    fn parses_particles() {
        let e = particles(json!({"images": ["./assets/p1.png", "assets/p2.svg"], "preset": "drift",
            "density": 12, "speed": 1.5, "minSize": 10, "maxSize": 40, "rotate": false, "opacity": 0.5}))
        .unwrap();
        assert_eq!(
            e,
            vec![Effect::Particles {
                images: vec!["assets/p1.png".into(), "assets/p2.svg".into()],
                preset: "drift".into(),
                density: Some(12.0),
                speed: Some(1.5),
                min_size: Some(10.0),
                max_size: Some(40.0),
                rotate: Some(false),
                opacity: Some(0.5),
            }]
        );
        let e = particles(json!({})).unwrap();
        assert!(matches!(
            &e[0],
            Effect::Particles {
                density: None,
                rotate: None,
                ..
            }
        ));
    }

    #[test]
    fn refuses_bad_particles() {
        let nine: Vec<&str> = vec!["assets/p1.png"; 9];
        let cases = [
            (json!({"images": null}), "images"),
            (json!({"images": []}), "images"),
            (json!({"images": nine}), "images"),
            (json!({"images": ["assets/a.ogg"]}), "images[0]"),
            (json!({"preset": "spin"}), "preset"),
            (json!({"preset": null}), "preset"),
            (json!({"minSize": 3}), "minSize"),
            (json!({"maxSize": 97}), "maxSize"),
            (json!({"minSize": 30, "maxSize": 20}), "minSize"),
            (json!({"opacity": 0}), "opacity"),
            (json!({"density": 31}), "density"),
            (json!({"speed": 3.5}), "speed"),
            (json!({"rotate": "yes"}), "rotate"),
        ];
        for (extra, field) in cases {
            let m = msg(particles(extra.clone()));
            assert!(m.contains(&format!("effects[0].{field}")), "{extra}: {m}");
        }
    }

    #[test]
    fn a_second_particles_effect_is_ignored() {
        let mut w = Vec::new();
        let one = json!({"type": "particles", "images": ["assets/p1.png"], "preset": "fall"});
        let e = parse_effects(&json!([one, one]), &files(&MEDIA), &mut w).unwrap();
        assert_eq!(e.len(), 1);
        assert_eq!(
            w,
            vec!["effects[1]: a second \"particles\" effect was ignored"]
        );
    }

    #[test]
    fn unknown_layout_and_sound_keys_warn() {
        let mut w = Vec::new();
        let l = parse_layout(&json!({"grid": 3, "nav": "compact"}), &mut w).unwrap();
        assert_eq!(l.nav.as_deref(), Some("compact"));
        let s = parse_sounds(
            &json!({"hover\u{202e}": "assets/a.ogg", "click": "assets/c.wav"}),
            &files(&MEDIA),
            &mut w,
        )
        .unwrap();
        assert_eq!(s.click.as_deref(), Some("assets/c.wav"));
        assert_eq!(w.len(), 2, "{w:?}");
        assert!(w[0].contains("layout.grid"), "{w:?}");
        assert!(
            w[1].contains("sounds.hover") && !w[1].contains('\u{202e}'),
            "{w:?}"
        );
    }
}
