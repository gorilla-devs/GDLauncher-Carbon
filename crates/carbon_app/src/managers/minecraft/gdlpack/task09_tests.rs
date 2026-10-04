use super::*;
use sha2::{Digest, Sha512};
use std::io::Write;
#[test]
fn task09_bundled_large_actual_zip_and_small_control() {
    for size in [1024, 64 * 1024 * 1024 + 1] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pack.gdlpack");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        zip.start_file(
            "overrides/mods/benign.txt",
            zip::write::FileOptions::<()>::default()
                .compression_method(zip::CompressionMethod::Stored),
        )
        .unwrap();
        let chunk = [b'x'; 65536];
        let mut hasher = Sha512::new();
        let mut remaining = size;
        while remaining > 0 {
            let n = remaining.min(chunk.len());
            zip.write_all(&chunk[..n]).unwrap();
            hasher.update(&chunk[..n]);
            remaining -= n;
        }
        zip.finish().unwrap();
        let hash = FileHashes {
            sha512: hex::encode(hasher.finalize()),
            sha1: String::new(),
            murmur2: 0,
        };
        assert_eq!(
            find_bundled_files(&path, "overrides", None, &[hash]).unwrap(),
            vec![true],
            "matching bundled bytes must work at size {size}"
        );
    }
}

#[test]
fn task09_bundled_budget_missing_decode_and_eligibility_are_distinct() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pack.gdlpack");
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
    let opts = zip::write::FileOptions::<()>::default();
    for (name, content) in [
        ("overrides/common", "abc"),
        ("custom-client/only", "test"),
        ("server-overrides/server", "server"),
        ("overrides-extra/no", "extra"),
        ("overrides/../escape", "escape"),
    ] {
        zip.start_file(name, opts).unwrap();
        zip.write_all(content.as_bytes()).unwrap();
    }
    zip.add_symlink("overrides/link", "link", opts).unwrap();
    zip.finish().unwrap();
    let hash = |bytes: &[u8]| FileHashes {
        sha512: hex::encode(Sha512::digest(bytes)).to_uppercase(),
        sha1: String::new(),
        murmur2: 0,
    };
    assert_eq!(
        find_bundled_files(
            &path,
            "overrides",
            Some("custom-client"),
            &[hash(b"test"), hash(b"abc")]
        )
        .unwrap(),
        [true, true]
    );
    let error = find_bundled_files_with_budget(
        &path,
        "overrides",
        Some("custom-client"),
        &[hash(b"test")],
        6,
    )
    .unwrap_err();
    assert!(error.to_string().contains("work budget exceeded"));
    for bytes in [
        b"server".as_slice(),
        b"extra",
        b"escape",
        b"link",
        b"missing",
    ] {
        assert_eq!(
            find_bundled_files(&path, "overrides", Some("custom-client"), &[hash(bytes)]).unwrap(),
            [false]
        );
    }
    std::fs::write(&path, b"not a ZIP").unwrap();
    assert!(
        find_bundled_files(&path, "overrides", None, &[hash(b"test")])
            .unwrap_err()
            .to_string()
            .contains("Decoding")
    );
    // No unresolved members means no fallback archive access, as after complete
    // platform resolution (the preparation caller also skips the entire block).
    assert!(
        find_bundled_files(&dir.path().join("absent"), "overrides", None, &[])
            .unwrap()
            .is_empty()
    );
}

#[test]
fn task09_bundled_crc_read_refusal_is_not_missing_content() {
    use std::io::{Seek, SeekFrom};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pack.gdlpack");
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
    zip.start_file(
        "overrides/file",
        zip::write::FileOptions::<()>::default().compression_method(zip::CompressionMethod::Stored),
    )
    .unwrap();
    zip.write_all(b"test").unwrap();
    zip.finish().unwrap();
    let offset = zip::ZipArchive::new(std::fs::File::open(&path).unwrap())
        .unwrap()
        .by_index(0)
        .unwrap()
        .data_start();
    let mut file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
    file.seek(SeekFrom::Start(offset)).unwrap();
    file.write_all(b"b").unwrap();
    drop(file);
    let hash = FileHashes {
        sha512: hex::encode(Sha512::digest(b"test")),
        sha1: String::new(),
        murmur2: 0,
    };
    let error = find_bundled_files(&path, "overrides", None, &[hash]).unwrap_err();
    assert!(error.to_string().contains("Reading bundled fallback entry"));
}
