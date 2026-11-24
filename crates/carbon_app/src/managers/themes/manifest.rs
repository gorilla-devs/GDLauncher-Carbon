use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::Value;

use super::error::ThemeError;
use super::extras::{
    Background, Decoration, Effect, Layout, Sounds, parse_background, parse_decorations,
    parse_effects, parse_layout, parse_sounds,
};
use super::files::{
    FileKind, ThemeFiles, ThemeLimits, clean_text, is_blank_text, is_valid_theme_id,
    normalize_rel_path, shown, shown_head_tail,
};
use super::season::{ManifestSeason, parse_season};

pub const SUPPORTED_THEME_API: u32 = 1;

pub const PALETTE_TOKENS: [&str; 86] = [
    "accent",
    "primary",
    "darkSlate-50",
    "darkSlate-100",
    "darkSlate-200",
    "darkSlate-300",
    "darkSlate-400",
    "darkSlate-500",
    "darkSlate-600",
    "darkSlate-700",
    "darkSlate-800",
    "darkSlate-900",
    "lightSlate-50",
    "lightSlate-100",
    "lightSlate-200",
    "lightSlate-300",
    "lightSlate-400",
    "lightSlate-500",
    "lightSlate-600",
    "lightSlate-700",
    "lightSlate-800",
    "lightSlate-900",
    "darkGray-50",
    "darkGray-100",
    "darkGray-200",
    "darkGray-300",
    "darkGray-400",
    "darkGray-500",
    "darkGray-600",
    "darkGray-700",
    "darkGray-800",
    "darkGray-900",
    "lightGray-50",
    "lightGray-100",
    "lightGray-200",
    "lightGray-300",
    "lightGray-400",
    "lightGray-500",
    "lightGray-600",
    "lightGray-700",
    "lightGray-800",
    "lightGray-900",
    "primary-50",
    "primary-100",
    "primary-200",
    "primary-300",
    "primary-400",
    "primary-500",
    "primary-600",
    "primary-700",
    "primary-800",
    "primary-900",
    "red-50",
    "red-100",
    "red-200",
    "red-300",
    "red-400",
    "red-500",
    "red-600",
    "red-700",
    "red-800",
    "red-900",
    "yellow-50",
    "yellow-100",
    "yellow-200",
    "yellow-300",
    "yellow-400",
    "yellow-500",
    "yellow-600",
    "yellow-700",
    "yellow-800",
    "yellow-900",
    "green-50",
    "green-100",
    "green-200",
    "green-300",
    "green-400",
    "green-500",
    "green-600",
    "green-700",
    "green-800",
    "green-900",
    "brands-curseforge",
    "brands-modrinth",
    "brands-discord",
    "brands-bisecthosting",
];

pub const APP_LANGUAGES: [&str; 7] = [
    "english",
    "italian",
    "japanese",
    "russian",
    "spanish",
    "simplified_chinese",
    "french",
];

#[derive(Debug, Clone, PartialEq)]
pub struct ManifestFont {
    pub family: String,
    pub src: String,
    pub weight: Option<String>,
    pub style: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ManifestTokens {
    pub font: Option<String>,
    pub font_mono: Option<String>,
    pub radius_scale: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidManifest {
    pub id: String,
    pub name: String,
    pub author: String,
    pub version: String,
    pub description: Option<String>,
    pub names: BTreeMap<String, String>,
    pub descriptions: BTreeMap<String, String>,
    pub preview: Option<String>,
    pub colors: BTreeMap<String, String>,
    pub fonts: Vec<ManifestFont>,
    pub tokens: ManifestTokens,
    pub css: Option<String>,
    pub background: Option<Background>,
    pub decorations: Vec<Decoration>,
    pub effects: Vec<Effect>,
    pub layout: Layout,
    pub sounds: Sounds,
    pub season: Option<ManifestSeason>,
    pub warnings: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawManifest {
    #[serde(default)]
    #[allow(dead_code)]
    theme_api: Value,
    id: Option<String>,
    name: Option<String>,
    author: Option<String>,
    version: Option<String>,
    description: Option<String>,
    #[serde(default)]
    names: BTreeMap<String, String>,
    #[serde(default)]
    descriptions: BTreeMap<String, String>,
    preview: Option<String>,
    #[serde(default)]
    colors: BTreeMap<String, String>,
    #[serde(default)]
    fonts: Vec<RawFont>,
    tokens: Option<RawTokens>,
    css: Option<String>,
    background: Option<Value>,
    decorations: Option<Value>,
    effects: Option<Value>,
    layout: Option<Value>,
    sounds: Option<Value>,
    season: Option<Value>,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

#[derive(Deserialize)]
struct RawFont {
    family: Option<String>,
    src: Option<String>,
    weight: Option<String>,
    style: Option<String>,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

#[derive(Deserialize)]
struct RawTokens {
    font: Option<String>,
    #[serde(rename = "font-mono")]
    font_mono: Option<String>,
    #[serde(rename = "radius-scale")]
    radius_scale: Option<f64>,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

fn invalid<T>(msg: impl Into<String>) -> Result<T, ThemeError> {
    Err(ThemeError::Invalid(msg.into()))
}

fn required(value: Option<String>, field: &str, max: usize) -> Result<String, ThemeError> {
    let Some(value) = value else {
        return invalid(format!("manifest.json: missing required field \"{field}\""));
    };
    let cleaned = clean_text(&value);
    let trimmed = cleaned.trim();
    let len = trimmed.chars().count();
    if is_blank_text(trimmed) || len > max {
        return invalid(format!("{field} must be 1 to {max} characters"));
    }
    Ok(trimmed.to_string())
}

fn optional_text(
    value: Option<String>,
    field: &str,
    max: usize,
) -> Result<Option<String>, ThemeError> {
    match value {
        None => Ok(None),
        Some(v) => {
            let v = clean_text(&v);
            if v.chars().count() > max {
                return invalid(format!("{field} must be at most {max} characters"));
            }
            Ok(Some(v))
        }
    }
}

pub(super) fn normalize_color(raw: &str) -> Option<String> {
    let parts: Vec<&str> = raw.split_whitespace().collect();
    if parts.len() != 3 {
        return None;
    }
    let mut out = Vec::new();
    for p in parts {
        if p.is_empty() || p.len() > 3 || !p.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let n: u32 = p.parse().ok()?;
        if n > 255 {
            return None;
        }
        out.push(n.to_string());
    }
    Some(out.join(" "))
}

fn valid_weight(w: &str) -> bool {
    if w == "normal" || w == "bold" {
        return true;
    }
    let parts: Vec<&str> = w.split(' ').collect();
    (1..=2).contains(&parts.len())
        && parts
            .iter()
            .all(|p| (1..=4).contains(&p.len()) && p.bytes().all(|b| b.is_ascii_digit()))
}

fn valid_family(f: &str) -> bool {
    let len = f.chars().count();
    (1..=64).contains(&len)
        && !f
            .chars()
            .any(|c| c.is_control() || "\"'\\;{}()<>".contains(c))
}

fn valid_font_stack(s: &str) -> bool {
    s.chars().count() <= 300 && !s.chars().any(|c| c.is_control() || ";{}()\\<>".contains(c))
}

pub(super) fn check_path(
    raw: &str,
    field: &str,
    kind: FileKind,
    kind_name: &str,
    files: &ThemeFiles,
) -> Result<String, ThemeError> {
    let Some(rel) = normalize_rel_path(raw) else {
        return invalid(format!(
            "{field} \"{}\" isn't a valid relative path",
            shown(raw)
        ));
    };
    if !files.files.contains_key(&rel) {
        return invalid(format!("{field} \"{}\" isn't in the theme", shown(raw)));
    }
    if files.kind_of(&rel) != Some(kind) {
        return invalid(format!("{field} \"{}\" must be {kind_name}", shown(raw)));
    }
    Ok(rel)
}

/// Parses a built-in theme's manifest, keeping and validating its `season`.
pub fn parse_builtin_manifest(
    bytes: &[u8],
    files: &ThemeFiles,
    limits: &ThemeLimits,
) -> Result<ValidManifest, ThemeError> {
    parse_manifest_as(bytes, files, limits, false)
}

/// Parses an installed theme's manifest. Only built-in themes have a season,
/// so it is not validated, only reported as ignored. This is the default;
/// use `parse_builtin_manifest` for the app's own themes.
pub fn parse_manifest(
    bytes: &[u8],
    files: &ThemeFiles,
    limits: &ThemeLimits,
) -> Result<ValidManifest, ThemeError> {
    parse_manifest_as(bytes, files, limits, true)
}

fn parse_manifest_as(
    bytes: &[u8],
    files: &ThemeFiles,
    limits: &ThemeLimits,
    installed: bool,
) -> Result<ValidManifest, ThemeError> {
    if bytes.len() as u64 > limits.max_manifest_bytes {
        return invalid(format!(
            "manifest.json is larger than {} KB",
            limits.max_manifest_bytes / 1024
        ));
    }
    // Windows PowerShell 5.1 and some editors write a UTF-8 byte order mark.
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let value: Value = serde_json::from_slice(bytes).map_err(|e| {
        ThemeError::Invalid(format!(
            "manifest.json: {}",
            shown_head_tail(&e.to_string())
        ))
    })?;
    let Some(obj) = value.as_object() else {
        return invalid("manifest.json must be a JSON object");
    };

    match obj.get("themeApi") {
        Some(Value::Number(n)) => match n.as_u64() {
            Some(v) if v > SUPPORTED_THEME_API as u64 => {
                return Err(ThemeError::TooNew {
                    required: v,
                    supported: SUPPORTED_THEME_API,
                });
            }
            Some(v) if v >= 1 => {}
            _ => return invalid("themeApi must be a positive whole number"),
        },
        _ => return invalid("themeApi must be a positive whole number"),
    }

    let raw: RawManifest = serde_json::from_value(value).map_err(|e| {
        ThemeError::Invalid(format!(
            "manifest.json: {}",
            shown_head_tail(&e.to_string())
        ))
    })?;

    let mut warnings = Vec::new();

    let id = raw.id.unwrap_or_default();
    if !is_valid_theme_id(&id) {
        return invalid(
            "id must be 1 to 64 characters of a-z, 0-9 and single \"-\", not starting or ending with \"-\"",
        );
    }
    let name = required(raw.name, "name", 64)?;
    let author = required(raw.author, "author", 64)?;
    let version = required(raw.version, "version", 32)?;
    let description = optional_text(raw.description, "description", 300)?;

    let mut translated = |map: BTreeMap<String, String>,
                          field: &str,
                          max: usize,
                          min: usize|
     -> Result<BTreeMap<String, String>, ThemeError> {
        let mut out = BTreeMap::new();
        for (lang, text) in map {
            if !APP_LANGUAGES.contains(&lang.as_str()) {
                warnings.push(format!(
                    "{field}.{} isn't an app language and was ignored",
                    shown(&lang)
                ));
                continue;
            }
            let cleaned = clean_text(&text);
            let text = cleaned.trim();
            // A required text with nothing visible (as `required` checks) would
            // show as a blank name in that language, so it is dropped.
            if min > 0 && is_blank_text(text) {
                warnings.push(format!("{field}.{} is blank and was ignored", shown(&lang)));
                continue;
            }
            let len = text.chars().count();
            if len < min || len > max {
                return invalid(format!(
                    "{field}.{} must be {min} to {max} characters",
                    shown(&lang)
                ));
            }
            out.insert(lang, text.to_string());
        }
        Ok(out)
    };
    let names = translated(raw.names, "names", 64, 1)?;
    let descriptions = translated(raw.descriptions, "descriptions", 300, 0)?;

    let preview = raw
        .preview
        .as_deref()
        .map(|p| check_path(p, "preview", FileKind::Image, "an image", files))
        .transpose()?;

    let mut colors = BTreeMap::new();
    for (token, value) in raw.colors {
        if !PALETTE_TOKENS.contains(&token.as_str()) {
            warnings.push(format!(
                "colors.{} isn't a palette token and was ignored",
                shown(&token)
            ));
            continue;
        }
        let Some(norm) = normalize_color(&value) else {
            return invalid(format!(
                "colors.{} must be three numbers from 0 to 255, like \"30 33 41\"",
                shown(&token)
            ));
        };
        colors.insert(token, norm);
    }

    if raw.fonts.len() > 32 {
        return invalid("fonts can have at most 32 entries");
    }
    let mut fonts = Vec::new();
    for (i, f) in raw.fonts.into_iter().enumerate() {
        for key in f.extra.keys() {
            warnings.push(format!(
                "fonts[{i}].{} isn't a font key and was ignored",
                shown(key)
            ));
        }
        let Some(family) = f.family else {
            return invalid(format!("fonts[{i}].family is required"));
        };
        let family = family.trim().to_string();
        if !valid_family(&family) {
            return invalid(format!(
                "fonts[{i}].family must be 1 to 64 characters without quotes, brackets, semicolons or backslashes"
            ));
        }
        let Some(src) = f.src else {
            return invalid(format!("fonts[{i}].src is required"));
        };
        let src = check_path(
            &src,
            &format!("fonts[{i}].src"),
            FileKind::Font,
            "a font file",
            files,
        )?;
        if let Some(w) = &f.weight {
            if !valid_weight(w) {
                return invalid(format!(
                    "fonts[{i}].weight must be normal, bold, or numbers like \"400\" or \"100 900\""
                ));
            }
        }
        if let Some(s) = &f.style {
            if !matches!(s.as_str(), "normal" | "italic" | "oblique") {
                return invalid(format!(
                    "fonts[{i}].style must be normal, italic or oblique"
                ));
            }
        }
        fonts.push(ManifestFont {
            family,
            src,
            weight: f.weight,
            style: f.style,
        });
    }

    let mut tokens = ManifestTokens::default();
    if let Some(t) = raw.tokens {
        for key in t.extra.keys() {
            warnings.push(format!(
                "tokens.{} isn't a token and was ignored",
                shown(key)
            ));
        }
        for (field, value) in [("font", &t.font), ("font-mono", &t.font_mono)] {
            if let Some(v) = value {
                if !valid_font_stack(v) {
                    return invalid(format!(
                        "tokens.{field} must be at most 300 characters without ; {{ }} ( ) \\ < >"
                    ));
                }
            }
        }
        if let Some(r) = t.radius_scale {
            if !r.is_finite() || !(0.0..=4.0).contains(&r) {
                return invalid("tokens.radius-scale must be a number from 0 to 4");
            }
        }
        tokens = ManifestTokens {
            font: t.font,
            font_mono: t.font_mono,
            radius_scale: t.radius_scale,
        };
    }

    let css = raw
        .css
        .as_deref()
        .map(|p| check_path(p, "css", FileKind::Css, "a .css file", files))
        .transpose()?;
    if css.is_none() && files.files.contains_key("theme.css") {
        warnings.push("theme.css isn't used because the manifest has no \"css\" key".to_string());
    }

    let background = raw
        .background
        .as_ref()
        .map(|v| parse_background(v, files, &mut warnings))
        .transpose()?;
    let decorations = match &raw.decorations {
        Some(v) => parse_decorations(v, files, &mut warnings)?,
        None => Vec::new(),
    };
    let effects = match &raw.effects {
        Some(v) => parse_effects(v, files, &mut warnings)?,
        None => Vec::new(),
    };
    let layout = match &raw.layout {
        Some(v) => parse_layout(v, &mut warnings)?,
        None => Layout::default(),
    };
    let sounds = match &raw.sounds {
        Some(v) => parse_sounds(v, files, &mut warnings)?,
        None => Sounds::default(),
    };

    let season = match &raw.season {
        Some(_) if installed => {
            warnings.push("season is only used by built-in themes and was ignored".to_string());
            None
        }
        Some(v) => Some(parse_season(v, Some(files), &mut warnings)?),
        None => None,
    };

    for key in raw.extra.keys() {
        warnings.push(format!(
            "\"{}\" isn't a manifest key and was ignored",
            shown(key)
        ));
    }

    Ok(ValidManifest {
        id,
        name,
        author,
        version,
        description,
        names,
        descriptions,
        preview,
        colors,
        fonts,
        tokens,
        css,
        background,
        decorations,
        effects,
        layout,
        sounds,
        season,
        warnings,
    })
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

    fn parse(v: Value, f: &[&str]) -> Result<ValidManifest, ThemeError> {
        parse_manifest(v.to_string().as_bytes(), &files(f), &ThemeLimits::DEFAULT)
    }

    fn base() -> Value {
        json!({"themeApi": 1, "id": "my-theme", "name": "My", "author": "me", "version": "1.0.0"})
    }

    fn with(extra: Value) -> Value {
        let mut b = base();
        for (k, v) in extra.as_object().unwrap() {
            b[k] = v.clone();
        }
        b
    }

    fn msg(r: Result<ValidManifest, ThemeError>) -> String {
        match r {
            Err(ThemeError::Invalid(m)) => m,
            other => panic!("expected Invalid, got {other:?}"),
        }
    }

    #[test]
    fn parses_a_full_manifest() {
        let v = json!({
            "themeApi": 1, "id": "my-theme", "name": " My Theme ", "author": "me", "version": "1.2.3",
            "description": "A theme",
            "names": {"italian": "Il mio"},
            "descriptions": {"italian": "Un tema"},
            "preview": "assets/p.png",
            "colors": {"primary": "8  17 42", "accent": "1 2 3"},
            "fonts": [{"family": "My Font", "src": "./fonts/a.woff2", "weight": "100 900", "style": "italic"}],
            "tokens": {"font": "My Font, sans-serif", "font-mono": "monospace", "radius-scale": 1.5},
            "css": "theme.css"
        });
        let m = parse(v, &["assets/p.png", "fonts/a.woff2", "theme.css"]).unwrap();
        assert_eq!(m.id, "my-theme");
        assert_eq!(m.name, "My Theme");
        assert_eq!(m.description.as_deref(), Some("A theme"));
        assert_eq!(m.names["italian"], "Il mio");
        assert_eq!(m.descriptions["italian"], "Un tema");
        assert_eq!(m.preview.as_deref(), Some("assets/p.png"));
        assert_eq!(m.colors["primary"], "8 17 42");
        assert_eq!(m.colors["accent"], "1 2 3");
        assert_eq!(
            m.fonts,
            vec![ManifestFont {
                family: "My Font".into(),
                src: "fonts/a.woff2".into(),
                weight: Some("100 900".into()),
                style: Some("italic".into()),
            }]
        );
        assert_eq!(m.tokens.font.as_deref(), Some("My Font, sans-serif"));
        assert_eq!(m.tokens.font_mono.as_deref(), Some("monospace"));
        assert_eq!(m.tokens.radius_scale, Some(1.5));
        assert_eq!(m.css.as_deref(), Some("theme.css"));
        assert!(m.warnings.is_empty());
    }

    #[test]
    fn warns_and_ignores_unknown_keys() {
        let v = with(json!({
            "widgets": "x",
            "colors": {"notAToken": "1 2 3"},
            "tokens": {"glow": 1},
            "names": {"klingon": "x"}
        }));
        let m = parse(v, &[]).unwrap();
        assert_eq!(m.warnings.len(), 4);
        assert!(m.colors.is_empty());
        assert!(m.names.is_empty());
        assert_eq!(m.tokens, ManifestTokens::default());
    }

    #[test]
    fn refuses_newer_theme_api() {
        let r = parse(json!({"themeApi": 2, "colors": 5}), &[]);
        assert!(matches!(
            r,
            Err(ThemeError::TooNew {
                required: 2,
                supported: 1
            })
        ));
    }

    #[test]
    fn refuses_missing_or_bad_required_fields() {
        let mut no_id = base();
        no_id.as_object_mut().unwrap().remove("id");
        assert!(msg(parse(no_id, &[])).contains("id"));
        assert!(msg(parse(with(json!({"id": "Bad Id"})), &[])).contains("id"));
        assert!(msg(parse(with(json!({"name": " "})), &[])).contains("name"));
        assert!(msg(parse(with(json!({"themeApi": 0})), &[])).contains("themeApi"));
        assert!(msg(parse(with(json!({"themeApi": "1"})), &[])).contains("themeApi"));
    }

    #[test]
    fn refuses_bad_paths() {
        msg(parse(with(json!({"preview": "../x.png"})), &["x.png"]));
        msg(parse(with(json!({"preview": "assets/missing.png"})), &[]));
        msg(parse(
            with(json!({"fonts": [{"family": "A", "src": "a.png"}]})),
            &["a.png"],
        ));
        msg(parse(with(json!({"css": "style.txt"})), &["style.txt"]));
    }

    #[test]
    fn refuses_bad_values() {
        msg(parse(with(json!({"colors": {"primary": "300 0 0"}})), &[]));
        msg(parse(with(json!({"colors": {"primary": "#ff0000"}})), &[]));
        msg(parse(with(json!({"tokens": {"radius-scale": 9}})), &[]));
        msg(parse(
            with(json!({"tokens": {"font": "x; } body { display:none"}})),
            &[],
        ));
        msg(parse(
            with(json!({"fonts": [{"family": "Evil\"", "src": "a.woff2"}]})),
            &["a.woff2"],
        ));
        let m = msg(parse(with(json!({"colors": {"primary": "300 0 0"}})), &[]));
        assert!(m.contains("colors.primary"));
    }

    #[test]
    fn path_errors_show_a_sanitized_capped_value() {
        let long = format!("{}\u{7}x", "a".repeat(300));
        let m = msg(parse(with(json!({"preview": long})), &[]));
        assert!(!m.contains('\u{7}'), "{m}");
        assert!(m.len() < 250, "{m}");
        let m = msg(parse(with(json!({"preview": "a\u{1b}b.png"})), &[]));
        assert!(!m.contains('\u{1b}'), "{m}");
    }

    #[test]
    fn serde_type_errors_stay_short() {
        // a wrong-typed value is echoed by serde ("invalid type: string \"...\"")
        let long = "x".repeat(200 * 1024);
        let m = msg(parse(with(json!({"names": long})), &[]));
        assert!(m.starts_with("manifest.json: "), "{m}");
        assert!(m.chars().count() < 200, "{}", m.chars().count());
        assert!(m.ends_with("expected a map"), "{m}");
        let m = msg(parse(with(json!({"descriptions": long})), &[]));
        assert!(m.chars().count() < 200, "{}", m.chars().count());
        assert!(m.ends_with("expected a map"), "{m}");
    }

    #[test]
    fn text_fields_drop_control_and_bidi_characters() {
        let dirty = "A\u{202e}b\u{200b}c\u{7}d\u{2066}e";
        let m = parse(
            with(json!({
                "name": dirty, "author": dirty, "version": dirty,
                "description": dirty,
                "names": {"italian": dirty},
                "descriptions": {"italian": dirty}
            })),
            &[],
        )
        .unwrap();
        assert_eq!(m.name, "Abcde");
        assert_eq!(m.author, "Abcde");
        assert_eq!(m.version, "Abcde");
        assert_eq!(m.description.as_deref(), Some("Abcde"));
        assert_eq!(m.names["italian"], "Abcde");
        assert_eq!(m.descriptions["italian"], "Abcde");
        // joiners survive in display text
        let bear = "\u{1F43B}\u{200D}\u{2744}\u{FE0F} Polar Night";
        let m = parse(with(json!({"name": bear})), &[]).unwrap();
        assert_eq!(m.name, bear);
        // a name made only of such characters, or of joiners, is refused
        let m = msg(parse(with(json!({"name": " \u{200D}\u{200C} "})), &[]));
        assert!(m.contains("name must be"), "{m}");
        let m = msg(parse(with(json!({"name": "\u{202e}\u{200b}"})), &[]));
        assert!(m.contains("name must be"), "{m}");
    }

    #[test]
    fn blank_name_translations_are_dropped_with_a_warning() {
        let m = parse(
            with(json!({"names": {
                "italian": " \u{200D}\u{200C} ",
                "french": "",
                "spanish": "Mío"
            }})),
            &[],
        )
        .unwrap();
        assert_eq!(m.names.len(), 1, "{:?}", m.names);
        assert_eq!(m.names["spanish"], "Mío");
        assert!(
            m.warnings
                .contains(&"names.italian is blank and was ignored".to_string()),
            "{:?}",
            m.warnings
        );
        assert!(
            m.warnings
                .contains(&"names.french is blank and was ignored".to_string()),
            "{:?}",
            m.warnings
        );
    }

    #[test]
    fn untrusted_keys_are_sanitized_in_warnings() {
        let k = format!("evil\u{1b}[31m\n{}", "x".repeat(300));
        let v = with(json!({
            k.clone(): 1,
            "colors": {k.clone(): "1 2 3"},
            "tokens": {k.clone(): 1},
            "fonts": [{"family": "A", "src": "a.woff2", k.clone(): 1}],
            "names": {k.clone(): "x"},
            "background": {"src": "assets/scene.svg", k.clone(): 1},
            "decorations": [{
                "anchor": "panel", "edge": "left", "width": 10,
                "images": ["assets/a.svg"], k.clone(): 1
            }],
            "effects": [{"type": "snow", k.clone(): 1}]
        }));
        let m = parse(v, &["a.woff2", "assets/scene.svg", "assets/a.svg"]).unwrap();
        assert!(m.warnings.len() >= 8, "{:?}", m.warnings);
        for w in &m.warnings {
            assert!(!w.chars().any(char::is_control), "{w}");
            assert!(w.chars().count() <= 260, "{w}");
            assert!(w.contains('…'), "{w}");
        }
    }

    #[test]
    fn a_leading_byte_order_mark_is_ignored() {
        let mut bytes = b"\xEF\xBB\xBF".to_vec();
        bytes.extend_from_slice(base().to_string().as_bytes());
        let m = parse_manifest(&bytes, &files(&[]), &ThemeLimits::DEFAULT).unwrap();
        assert_eq!(m.id, "my-theme");
    }

    #[test]
    fn font_family_is_trimmed() {
        let m = parse(
            with(json!({"fonts": [{"family": "  My Font ", "src": "a.woff2"}]})),
            &["a.woff2"],
        )
        .unwrap();
        assert_eq!(m.fonts[0].family, "My Font");
    }

    #[test]
    fn warns_when_theme_css_is_unused() {
        let m = parse(base(), &["theme.css"]).unwrap();
        assert_eq!(
            m.warnings,
            vec!["theme.css isn't used because the manifest has no \"css\" key"]
        );
    }
}
