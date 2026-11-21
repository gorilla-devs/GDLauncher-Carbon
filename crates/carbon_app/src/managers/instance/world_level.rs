//! A world's in-game name, which lives in `Data.LevelName` of its `level.dat`.

use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use anyhow::{Context, bail};
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;

use crate::util::nbt::{self, Root, Tag};

/// A `level.dat` that inflates past this is refused rather than held in memory.
const MAX_LEVEL_DAT_SIZE: u64 = 64 * 1024 * 1024;

fn read_tree(path: &Path) -> anyhow::Result<Root> {
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let mut raw = Vec::new();
    GzDecoder::new(file)
        .take(MAX_LEVEL_DAT_SIZE + 1)
        .read_to_end(&mut raw)
        .with_context(|| format!("decompressing {}", path.display()))?;
    if raw.len() as u64 > MAX_LEVEL_DAT_SIZE {
        bail!(
            "{} is larger than {MAX_LEVEL_DAT_SIZE} bytes decompressed",
            path.display()
        );
    }
    Ok(nbt::read(&raw)?)
}

#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn read_level_name(world: &Path) -> anyhow::Result<Option<String>> {
    let root = read_tree(&world.join("level.dat"))?;
    Ok(
        match root.tag.get("Data").and_then(|data| data.get("LevelName")) {
            Some(Tag::String(raw)) => nbt::decode_string(raw),
            _ => None,
        },
    )
}

#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn write_level_name(world: &Path, name: &str) -> anyhow::Result<()> {
    let target = world.join("level.dat");
    let mut root = read_tree(&target)?;

    let value = Tag::String(nbt::encode_string(name));
    let Some(Tag::Compound(entries)) = root.tag.get_mut("Data") else {
        bail!("{} has no Data compound", target.display());
    };
    match entries.iter_mut().find(|(key, _)| key == b"LevelName") {
        Some((_, tag)) => *tag = value,
        None => entries.push((b"LevelName".to_vec(), value)),
    }

    // The new file sits beside level.dat so the final rename stays on one
    // volume; dropping it on any error removes it.
    let mut tmp = tempfile::Builder::new()
        .prefix("level.dat.")
        .suffix(".tmp")
        .tempfile_in(world)
        .context("creating the new level.dat")?;
    let mut encoder = GzEncoder::new(tmp.as_file_mut(), Compression::default());
    encoder.write_all(&nbt::write(&root))?;
    encoder.finish()?;

    if read_tree(tmp.path())? != root {
        bail!("the new level.dat did not read back as intended");
    }
    tmp.as_file().sync_all()?;
    tmp.persist(&target)
        .map_err(|e| e.error)
        .context("replacing level.dat")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(s: &str) -> Vec<u8> {
        nbt::encode_string(s)
    }

    fn tree(name: Option<&str>, extra: Vec<(&str, Tag)>) -> Root {
        let mut data: Vec<(Vec<u8>, Tag)> = extra
            .into_iter()
            .map(|(k, v)| (k.as_bytes().to_vec(), v))
            .collect();
        if let Some(name) = name {
            data.push((b"LevelName".to_vec(), Tag::String(text(name))));
        }
        Root {
            name: vec![],
            tag: Tag::Compound(vec![(b"Data".to_vec(), Tag::Compound(data))]),
        }
    }

    fn gzip(bytes: &[u8]) -> Vec<u8> {
        let mut enc = GzEncoder::new(Vec::new(), Compression::default());
        enc.write_all(bytes).unwrap();
        enc.finish().unwrap()
    }

    fn level_dat(name: Option<&str>, extra: Vec<(&str, Tag)>) -> Vec<u8> {
        gzip(&nbt::write(&tree(name, extra)))
    }

    fn world_with(bytes: &[u8]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("level.dat"), bytes).unwrap();
        dir
    }

    #[test]
    fn the_name_is_read() {
        let dir = world_with(&level_dat(Some("Skyblock with friends"), vec![]));
        assert_eq!(
            read_level_name(dir.path()).unwrap(),
            Some("Skyblock with friends".to_string())
        );
    }

    #[test]
    fn a_world_without_a_name_tag_has_no_name() {
        let dir = world_with(&level_dat(None, vec![("version", Tag::Int(1))]));
        assert_eq!(read_level_name(dir.path()).unwrap(), None);
    }

    #[test]
    fn a_name_that_is_not_a_string_has_no_name() {
        let dir = world_with(&level_dat(None, vec![("LevelName", Tag::Int(5))]));
        assert_eq!(read_level_name(dir.path()).unwrap(), None);
    }

    #[test]
    fn a_truncated_file_is_an_error() {
        let bytes = level_dat(Some("x"), vec![("blob", Tag::ByteArray(vec![7; 4096]))]);
        let dir = world_with(&bytes[..bytes.len() / 2]);
        assert!(read_level_name(dir.path()).is_err());
    }

    #[test]
    fn a_file_that_is_not_gzip_is_an_error() {
        let dir = world_with(b"this is not a level.dat");
        assert!(read_level_name(dir.path()).is_err());
    }

    #[test]
    fn a_missing_level_dat_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read_level_name(dir.path()).is_err());
    }

    fn every_type() -> Vec<(&'static str, Tag)> {
        let mut extra = vec![
            ("byte", Tag::Byte(-3)),
            ("short", Tag::Short(300)),
            ("int", Tag::Int(-70000)),
            ("long", Tag::Long(1 << 40)),
            ("float", Tag::Float(0.25)),
            ("double", Tag::Double(-2.5)),
            ("string", Tag::String(text("héllo"))),
            (
                "list",
                Tag::List {
                    element: 3,
                    items: vec![Tag::Int(1), Tag::Int(2)],
                },
            ),
            (
                "empty_list",
                Tag::List {
                    element: 0,
                    items: vec![],
                },
            ),
            (
                "compound",
                Tag::Compound(vec![(b"inner".to_vec(), Tag::Byte(1))]),
            ),
            ("ints", Tag::IntArray(vec![1, -2, 3])),
            ("longs", Tag::LongArray(vec![4, -5])),
        ];
        for (i, name) in [
            "extra_a", "extra_b", "extra_c", "extra_d", "extra_e", "extra_f", "extra_g", "extra_h",
        ]
        .into_iter()
        .enumerate()
        {
            extra.push((name, Tag::Int(i as i32)));
        }
        // Pseudo-random so the 2 MiB array does not collapse to nothing when compressed.
        let mut seed = 0x9E37_79B9u32;
        let blob = (0..2 * 1024 * 1024)
            .map(|_| {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                (seed >> 24) as u8
            })
            .collect();
        extra.push(("blob", Tag::ByteArray(blob)));
        extra
    }

    #[test]
    fn writing_the_name_changes_nothing_else() {
        let before = tree(Some("Old"), every_type());
        let dir = world_with(&gzip(&nbt::write(&before)));
        std::fs::write(dir.path().join("level.dat_old"), b"previous").unwrap();

        write_level_name(dir.path(), "Season 2 😀").unwrap();

        let mut expected = before;
        let Some(Tag::Compound(data)) = expected.tag.get_mut("Data") else {
            unreachable!()
        };
        data.iter_mut().find(|(k, _)| k == b"LevelName").unwrap().1 =
            Tag::String(text("Season 2 😀"));
        assert_eq!(read_tree(&dir.path().join("level.dat")).unwrap(), expected);
        assert_eq!(
            read_level_name(dir.path()).unwrap(),
            Some("Season 2 😀".to_string())
        );

        let mut files: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        files.sort();
        assert_eq!(files, ["level.dat", "level.dat_old"]);
        assert_eq!(
            std::fs::read(dir.path().join("level.dat_old")).unwrap(),
            b"previous"
        );
    }

    #[test]
    fn a_name_tag_is_added_when_the_world_has_none() {
        let dir = world_with(&level_dat(None, vec![("version", Tag::Int(1))]));
        write_level_name(dir.path(), "Fresh").unwrap();
        assert_eq!(
            read_level_name(dir.path()).unwrap(),
            Some("Fresh".to_string())
        );
        let root = read_tree(&dir.path().join("level.dat")).unwrap();
        assert_eq!(
            root.tag.get("Data").unwrap().get("version"),
            Some(&Tag::Int(1))
        );
    }

    #[test]
    fn a_world_without_data_cannot_be_renamed() {
        let root = Root {
            name: vec![],
            tag: Tag::Compound(vec![]),
        };
        let dir = world_with(&gzip(&nbt::write(&root)));
        assert!(write_level_name(dir.path(), "x").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn a_failed_write_leaves_the_file_as_it_was() {
        use std::os::unix::fs::PermissionsExt;

        let bytes = level_dat(Some("Old"), vec![]);
        let dir = world_with(&bytes);
        let mut perms = std::fs::metadata(dir.path()).unwrap().permissions();
        perms.set_mode(0o555);
        std::fs::set_permissions(dir.path(), perms.clone()).unwrap();

        let result = write_level_name(dir.path(), "New");

        perms.set_mode(0o755);
        std::fs::set_permissions(dir.path(), perms).unwrap();
        assert!(result.is_err());
        assert_eq!(std::fs::read(dir.path().join("level.dat")).unwrap(), bytes);
    }
}
