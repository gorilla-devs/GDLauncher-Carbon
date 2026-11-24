use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

use super::error::ThemeError;
use super::files::{
    Failure, FileKind, ThemeLimits, check_case_collisions, check_text_size, file_kind,
    is_ignored_entry, normalize_rel_path, shown,
};

fn invalid(msg: impl Into<String>) -> Failure {
    Failure::Theme(ThemeError::Invalid(msg.into()))
}

/// A disk or caller problem, never a defect in the theme.
fn io(msg: impl Into<String>) -> Failure {
    Failure::Io(msg.into())
}

/// Extracts an untrusted `.gdltheme` zip into `dest`, which must not exist yet or
/// must be empty. Blocking: callers wrap it in `spawn_blocking`. On error `dest` may
/// hold partial output, so the caller removes it.
pub fn extract_theme_zip(
    zip_path: &Path,
    dest: &Path,
    limits: &ThemeLimits,
) -> Result<(), Failure> {
    let not_a_zip = || invalid("this isn't a valid .gdltheme file");
    let damaged = || invalid("this .gdltheme file is damaged");
    let file = File::open(zip_path)
        .map_err(|e| io(format!("couldn't open the theme file: {}", e.kind())))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|_| not_a_zip())?;

    // (index, normalized path, declared size)
    let mut kept: Vec<(usize, String, u64)> = Vec::new();
    for i in 0..archive.len() {
        // Raw: no decryption or decompression is needed to read the metadata.
        let entry = archive.by_index_raw(i).map_err(|_| not_a_zip())?;
        // PowerShell's Compress-Archive writes backslashes.
        let name = entry.name().replace('\\', "/");
        let is_dir = entry.is_dir() || name.ends_with('/');
        if is_dir && name.chars().all(|c| c == '.' || c == '/') {
            continue;
        }
        let rel = normalize_rel_path(&name)
            .ok_or_else(|| invalid(format!("unsafe path in the theme file: {}", shown(&name))))?;
        if is_dir || is_ignored_entry(&rel) {
            continue;
        }
        if entry.is_symlink() {
            return Err(invalid(format!("symlinks aren't allowed: {}", shown(&rel))));
        }
        if entry.encrypted() {
            return Err(invalid(format!(
                "encrypted files aren't allowed: {}",
                shown(&rel)
            )));
        }
        kept.push((i, rel, entry.size()));
    }

    // A single wrapping folder, as made by "compress this folder".
    if !kept.iter().any(|(_, rel, _)| rel == "manifest.json") {
        if let Some((top, _)) = kept.first().and_then(|(_, rel, _)| rel.split_once('/')) {
            let prefix = format!("{top}/");
            let manifest = format!("{prefix}manifest.json");
            if kept.iter().all(|(_, rel, _)| rel.starts_with(&prefix))
                && kept.iter().any(|(_, rel, _)| *rel == manifest)
            {
                for (_, rel, _) in &mut kept {
                    rel.drain(..prefix.len());
                }
            }
        }
    }

    if kept.len() > limits.max_files {
        return Err(invalid(format!(
            "the theme has more than {} files",
            limits.max_files
        )));
    }

    let mut declared: u64 = 0;
    for (_, rel, size) in &kept {
        if file_kind(rel).is_none() {
            return Err(invalid(format!(
                "this file type isn't allowed: {}",
                shown(rel)
            )));
        }
        check_text_size(rel, *size, limits).map_err(invalid)?;
        declared = declared.saturating_add(*size);
    }
    check_case_collisions(kept.iter().map(|(_, rel, _)| rel.as_str())).map_err(invalid)?;
    if declared > limits.max_total_bytes {
        return Err(invalid("the theme is larger than 50 MB unpacked"));
    }

    if dest.exists() {
        let empty = dest.is_dir()
            && fs::read_dir(dest)
                .map(|mut d| d.next().is_none())
                .unwrap_or(false);
        if !empty {
            return Err(io("the destination folder isn't empty"));
        }
    }
    fs::create_dir_all(dest).map_err(|e| io(format!("couldn't create the theme folder: {e}")))?;

    let mut remaining = limits.max_total_bytes;
    for (i, rel, _) in &kept {
        let mut entry = archive.by_index(*i).map_err(|_| damaged())?;
        let out_path = dest.join(rel);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| io(format!("couldn't create the theme folder: {e}")))?;
        }
        let mut out = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&out_path)
            .map_err(|e| io(format!("couldn't write {rel}: {e}")))?;
        // Reading the entry (corrupt data, bad checksum) is a damaged theme;
        // writing it out is the disk's problem.
        let mut buf = [0u8; 64 * 1024];
        let mut copied: u64 = 0;
        loop {
            let n = match (&mut entry).read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(_) => return Err(damaged()),
            };
            copied += n as u64;
            if copied > remaining {
                return Err(invalid("the theme is larger than 50 MB unpacked"));
            }
            // A text file's declared size can lie; its real size can't.
            if file_kind(rel) == Some(FileKind::Text) {
                check_text_size(rel, copied, limits).map_err(invalid)?;
            }
            out.write_all(&buf[..n])
                .map_err(|e| io(format!("couldn't write {rel}: {}", e.kind())))?;
        }
        remaining -= copied;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    enum E<'a> {
        File(&'a str, &'a [u8]),
        Deflated(&'a str, &'a [u8]),
        Link(&'a str, &'a str),
        Dir(&'a str),
    }

    fn err_of(r: Result<(), Failure>) -> String {
        r.unwrap_err().to_string()
    }

    fn make(entries: &[E]) -> (tempfile::TempDir, std::path::PathBuf) {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("t.gdltheme");
        let mut z = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        let stored =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        for e in entries {
            match e {
                E::File(n, b) => {
                    z.start_file(*n, stored).unwrap();
                    z.write_all(b).unwrap();
                }
                E::Deflated(n, b) => {
                    z.start_file(
                        *n,
                        SimpleFileOptions::default()
                            .compression_method(zip::CompressionMethod::Deflated),
                    )
                    .unwrap();
                    z.write_all(b).unwrap();
                }
                E::Link(n, t) => z.add_symlink(*n, *t, stored).unwrap(),
                E::Dir(n) => z.add_directory(*n, stored).unwrap(),
            }
        }
        z.finish().unwrap();
        (d, path)
    }

    fn run(entries: &[E], limits: &ThemeLimits) -> (tempfile::TempDir, Result<(), Failure>) {
        let (d, zp) = make(entries);
        let dest = d.path().join("out");
        let r = extract_theme_zip(&zp, &dest, limits);
        (d, r)
    }

    fn listing(dir: &Path) -> Vec<String> {
        let mut v: Vec<String> = walkdir::WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| {
                e.path()
                    .strip_prefix(dir)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();
        v.sort();
        v
    }

    #[test]
    fn extracts_a_flat_theme() {
        let (d, r) = run(
            &[
                E::File("manifest.json", b"{}"),
                E::File("theme.css", b"a{}"),
                E::File("assets/a.png", &[1, 2, 3]),
            ],
            &ThemeLimits::DEFAULT,
        );
        r.unwrap();
        let out = d.path().join("out");
        assert_eq!(std::fs::read(out.join("manifest.json")).unwrap(), b"{}");
        assert_eq!(std::fs::read(out.join("theme.css")).unwrap(), b"a{}");
        assert_eq!(std::fs::read(out.join("assets/a.png")).unwrap(), [1, 2, 3]);
    }

    #[test]
    fn strips_a_wrapping_folder_and_skips_os_junk() {
        let (d, r) = run(
            &[
                E::File("Snow/manifest.json", b"{}"),
                E::File("Snow/assets/a.png", b"x"),
                E::File("__MACOSX/Snow/._a.png", b"x"),
                E::File("Snow/.DS_Store", b"x"),
                E::File("Snow/Thumbs.db", b"x"),
            ],
            &ThemeLimits::DEFAULT,
        );
        r.unwrap();
        assert_eq!(
            listing(&d.path().join("out")),
            ["assets/a.png", "manifest.json"]
        );
    }

    #[test]
    fn installs_a_zip_with_backslash_separators() {
        let (d, r) = run(
            &[
                E::File("manifest.json", b"{}"),
                E::File("assets\\a.png", b"x"),
            ],
            &ThemeLimits::DEFAULT,
        );
        r.unwrap();
        assert_eq!(
            listing(&d.path().join("out")),
            ["assets/a.png", "manifest.json"]
        );
    }

    #[test]
    fn rejects_zip_slip() {
        for bad in ["../evil.png", "/abs.png", "assets/../../x.png"] {
            let (d, r) = run(
                &[E::File("manifest.json", b"{}"), E::File(bad, b"x")],
                &ThemeLimits::DEFAULT,
            );
            assert!(err_of(r).contains("unsafe path"), "{bad}");
            assert!(!d.path().join("evil.png").exists());
            assert!(!d.path().join("x.png").exists());
        }
    }

    #[test]
    fn rejects_symlinks() {
        let (_d, r) = run(
            &[
                E::File("manifest.json", b"{}"),
                E::Link("assets/l.png", "/etc/passwd"),
            ],
            &ThemeLimits::DEFAULT,
        );
        assert!(r.unwrap_err().to_string().contains("symlink"));
    }

    #[test]
    fn rejects_disallowed_types() {
        for bad in ["run.exe", "x.js"] {
            let (_d, r) = run(
                &[E::File("manifest.json", b"{}"), E::File(bad, b"x")],
                &ThemeLimits::DEFAULT,
            );
            assert!(err_of(r).contains("isn't allowed"), "{bad}");
        }
    }

    #[test]
    fn rejects_windows_reserved_names() {
        let (_d, r) = run(
            &[
                E::File("manifest.json", b"{}"),
                E::File("assets/CON.png", b"x"),
            ],
            &ThemeLimits::DEFAULT,
        );
        assert!(err_of(r).contains("unsafe path"));
    }

    #[test]
    fn rejects_case_colliding_entries() {
        for (a, b) in [("a.png", "A.png"), ("theme.css", "Theme.css")] {
            let (_d, r) = run(
                &[
                    E::File("manifest.json", b"{}"),
                    E::File(a, b"x"),
                    E::File(b, b"y"),
                ],
                &ThemeLimits::DEFAULT,
            );
            assert!(err_of(r).contains("two files named"), "{a} vs {b}");
        }
    }

    #[test]
    fn rejects_folders_that_differ_only_by_case() {
        let (_d, r) = run(
            &[
                E::File("manifest.json", b"{}"),
                E::File("Assets/a.png", b"x"),
                E::File("assets/b.png", b"x"),
            ],
            &ThemeLimits::DEFAULT,
        );
        assert!(err_of(r).contains("two folders named"));
    }

    #[test]
    fn rejects_a_path_that_is_both_a_file_and_a_folder() {
        for (a, b) in [("a.png", "a.png/b.png"), ("A.PNG", "a.png/x.png")] {
            let (d, r) = run(
                &[
                    E::File("manifest.json", b"{}"),
                    E::File(a, b"x"),
                    E::File(b, b"y"),
                ],
                &ThemeLimits::DEFAULT,
            );
            assert!(err_of(r).contains("a file and a folder"), "{a} vs {b}");
            // Refused before anything is written.
            assert!(!d.path().join("out").exists());
        }
    }

    #[test]
    fn refuses_unsafe_folder_entries() {
        for bad in ["../x/", "/abs/"] {
            let (_d, r) = run(
                &[E::File("manifest.json", b"{}"), E::Dir(bad)],
                &ThemeLimits::DEFAULT,
            );
            assert!(err_of(r).contains("unsafe path"), "{bad}");
        }
        let (d, r) = run(
            &[E::File("manifest.json", b"{}"), E::Dir("assets/")],
            &ThemeLimits::DEFAULT,
        );
        r.unwrap();
        assert_eq!(listing(&d.path().join("out")), ["manifest.json"]);
    }

    #[test]
    fn error_messages_hide_control_characters_and_long_names() {
        let name = format!("../a\u{1b}[31m{}.png", "x".repeat(300));
        let (_d, r) = run(&[E::File(&name, b"x")], &ThemeLimits::DEFAULT);
        let msg = err_of(r);
        assert!(!msg.contains('\u{1b}'), "{msg:?}");
        assert!(msg.len() < 250, "{}", msg.len());
    }

    #[test]
    fn refuses_a_non_empty_destination() {
        let (d, zp) = make(&[E::File("manifest.json", b"{}")]);
        let dest = d.path().join("out");
        std::fs::create_dir(&dest).unwrap();
        std::fs::write(dest.join("old.txt"), "x").unwrap();
        let e = extract_theme_zip(&zp, &dest, &ThemeLimits::DEFAULT).unwrap_err();
        assert!(e.to_string().contains("isn't empty"), "{e}");
        // An existing empty folder is fine.
        std::fs::remove_file(dest.join("old.txt")).unwrap();
        extract_theme_zip(&zp, &dest, &ThemeLimits::DEFAULT).unwrap();
    }

    #[test]
    fn accepts_dot_slash_names() {
        let (d, r) = run(&[E::File("./manifest.json", b"{}")], &ThemeLimits::DEFAULT);
        r.unwrap();
        assert_eq!(listing(&d.path().join("out")), ["manifest.json"]);
    }

    #[test]
    fn refuses_encrypted_entries() {
        // Set the "encrypted" bit (general purpose flag bit 0) in every header.
        let (d, zp) = make(&[E::File("manifest.json", b"{}"), E::File("a.png", b"secret")]);
        let mut bytes = std::fs::read(&zp).unwrap();
        for i in 0..bytes.len() - 10 {
            if &bytes[i..i + 4] == b"PK\x01\x02" {
                bytes[i + 8] |= 1;
            } else if &bytes[i..i + 4] == b"PK\x03\x04" {
                bytes[i + 6] |= 1;
            }
        }
        std::fs::write(&zp, bytes).unwrap();
        let msg = err_of(extract_theme_zip(
            &zp,
            &d.path().join("out"),
            &ThemeLimits::DEFAULT,
        ));
        assert!(msg.contains("encrypted"), "{msg}");
    }

    fn patch_sizes(bytes: &mut [u8], new_size: u32) {
        let sig_at = |bytes: &[u8], sig: &[u8; 4]| -> Vec<usize> {
            (0..bytes.len().saturating_sub(4))
                .filter(|&i| &bytes[i..i + 4] == sig)
                .collect()
        };
        let mut patched = 0;
        for i in sig_at(bytes, b"PK\x01\x02") {
            bytes[i + 24..i + 28].copy_from_slice(&new_size.to_le_bytes());
            patched += 1;
        }
        for i in sig_at(bytes, b"PK\x03\x04") {
            bytes[i + 22..i + 26].copy_from_slice(&new_size.to_le_bytes());
            patched += 1;
        }
        assert!(patched >= 2);
    }

    #[test]
    fn extracts_a_licence_text_file_within_its_cap() {
        let (d, r) = run(
            &[
                E::File("manifest.json", b"{}"),
                E::File("fonts/OFL.txt", b"SIL Open Font License"),
            ],
            &ThemeLimits::DEFAULT,
        );
        r.unwrap();
        assert!(d.path().join("out/fonts/OFL.txt").is_file());

        let l = ThemeLimits {
            max_text_bytes: 4,
            ..ThemeLimits::DEFAULT
        };
        let (_d, r) = run(
            &[
                E::File("manifest.json", b"{}"),
                E::File("OFL.txt", b"too long"),
            ],
            &l,
        );
        assert!(err_of(r).contains("OFL.txt is larger than"));
    }

    #[test]
    fn a_text_file_with_a_forged_size_stops_at_its_cap() {
        let text = vec![b'a'; 10_000];
        let (d, zp) = make(&[E::Deflated("OFL.txt", &text)]);
        let mut bytes = std::fs::read(&zp).unwrap();
        patch_sizes(&mut bytes, 10);
        std::fs::write(&zp, bytes).unwrap();

        let l = ThemeLimits {
            max_text_bytes: 1_000,
            ..ThemeLimits::DEFAULT
        };
        let msg = err_of(extract_theme_zip(&zp, &d.path().join("out"), &l));
        assert!(msg.contains("OFL.txt is larger than"), "{msg}");
    }

    #[test]
    fn a_forged_size_header_cant_get_past_the_copy_cap() {
        let zeros = vec![0u8; 10_000];
        let (d, zp) = make(&[E::Deflated("a.png", &zeros)]);
        let mut bytes = std::fs::read(&zp).unwrap();
        patch_sizes(&mut bytes, 10);
        std::fs::write(&zp, bytes).unwrap();

        let l = ThemeLimits {
            max_total_bytes: 1_000,
            ..ThemeLimits::DEFAULT
        };
        let dest = d.path().join("out");
        let msg = err_of(extract_theme_zip(&zp, &dest, &l));
        assert!(msg.contains("larger than"), "{msg}");
        let written: u64 = walkdir::WalkDir::new(&dest)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| e.metadata().unwrap().len())
            .sum();
        assert!(written <= l.max_total_bytes + 1, "{written}");
    }

    #[test]
    fn rejects_too_many_files_and_oversized_content() {
        let l = ThemeLimits {
            max_files: 2,
            ..ThemeLimits::DEFAULT
        };
        let (_d, r) = run(
            &[
                E::File("manifest.json", b"{}"),
                E::File("a.png", b"x"),
                E::File("b.png", b"x"),
            ],
            &l,
        );
        assert!(r.is_err());

        let l = ThemeLimits {
            max_total_bytes: 1_000,
            ..ThemeLimits::DEFAULT
        };
        let zeros = vec![0u8; 10_000];
        let (_d, r) = run(
            &[
                E::File("manifest.json", b"{}"),
                E::Deflated("a.png", &zeros),
            ],
            &l,
        );
        assert!(r.is_err());
    }

    #[test]
    fn rejects_garbage() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("x.gdltheme");
        std::fs::write(&p, "hello").unwrap();
        let e = extract_theme_zip(&p, &d.path().join("out"), &ThemeLimits::DEFAULT).unwrap_err();
        assert!(e.to_string().contains("valid .gdltheme"), "{e}");
    }
}
