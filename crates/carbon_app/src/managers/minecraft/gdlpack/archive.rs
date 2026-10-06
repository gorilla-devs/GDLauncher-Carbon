use crate::api::translation::Translation;
use carbon_platforms::gdlauncher::manifest::schema::v1::Manifest;
use std::{
    io::Read,
    path::{Path, PathBuf},
};

pub(crate) fn inspect(path: &Path) -> Result<(Manifest, Option<(Vec<u8>, String)>), Translation> {
    let file =
        std::fs::File::open(path).map_err(|_| Translation::InstanceImportGdlpackMalformed)?;
    let mut zip =
        zip::ZipArchive::new(file).map_err(|_| Translation::InstanceImportGdlpackMalformed)?;
    let manifest = {
        let mut entry = zip
            .by_name("gdlpack.json")
            .map_err(|_| Translation::InstanceImportGdlpackMissingManifest)?;
        let mut data = Vec::new();
        entry
            .read_to_end(&mut data)
            .map_err(|_| Translation::InstanceImportGdlpackMalformedManifest)?;
        let manifest: Manifest = serde_json::from_slice(&data)
            .map_err(|_| Translation::InstanceImportGdlpackMalformedManifest)?;
        if manifest.format_version != 1 {
            return Err(Translation::InstanceImportGdlpackMalformedManifest);
        }
        if manifest.entries.iter().any(|entry| {
            matches!(
                entry,
                carbon_platforms::gdlauncher::manifest::schema::v1::PackFile::Optional(_)
            )
        }) {
            return Err(Translation::InstanceImportGdlpackOptionalUnsupported);
        }
        manifest
    };
    let icon = manifest.icon.as_ref().and_then(|path| {
        let mut entry = zip.by_name(path).ok()?;
        let mut data = Vec::new();
        entry.read_to_end(&mut data).ok()?;
        let ext = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png")
            .to_owned();
        Some((data, ext))
    });
    Ok((manifest, icon))
}

pub(crate) struct AdmittedArchive {
    file: tempfile::NamedTempFile,
    manifest: Manifest,
    icon: Option<(Vec<u8>, String)>,
}
impl AdmittedArchive {
    pub(crate) async fn capture(source: PathBuf, temp: PathBuf) -> anyhow::Result<Self> {
        tokio::task::spawn_blocking(move || {
            // Capture before validation: neither admission nor later preparation
            // may reopen the user's mutable source archive after deciding to accept it.
            std::fs::create_dir_all(&temp)?;
            let mut file = tempfile::NamedTempFile::new_in(temp)?;
            let mut source = std::fs::File::open(source)?;
            std::io::copy(&mut source, file.as_file_mut())?;
            let (manifest, icon) = inspect(file.path()).map_err(|reason| match reason {
                Translation::InstanceImportGdlpackOptionalUnsupported => anyhow::anyhow!(
                    "GDLPack optional entries are not supported. Use required platform entries and regular overrides."
                ),
                other => anyhow::anyhow!("GDLPack import refused: {other:?}"),
            })?;
            Ok(Self { file, manifest, icon })
        }).await?
    }
    pub(crate) fn manifest(&self) -> &Manifest {
        &self.manifest
    }
    pub(crate) fn icon(&self) -> &Option<(Vec<u8>, String)> {
        &self.icon
    }
    pub(crate) fn path(&self) -> &Path {
        self.file.path()
    }
    pub(crate) async fn install(self, setup: PathBuf) -> anyhow::Result<()> {
        // The worker retains the captured file through entered filesystem IO,
        // even if the awaiting request is dropped.
        tokio::task::spawn_blocking(move || {
            std::fs::create_dir_all(&setup)?;
            std::fs::copy(self.path(), setup.join("gdlpack"))?;
            Ok(())
        })
        .await?
    }
}

#[cfg(test)]
mod task40_tests {
    use super::*;
    use std::io::Write;
    fn pack(path: &Path, optional: bool, version: u32, content: &[u8]) {
        let mut z = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        let opts = zip::write::FileOptions::<()>::default();
        let entries = if optional {
            serde_json::json!([{
                "type":"optional", "description":"Shader support", "platforms":[{
                    "sha512":"AB12", "sha1":"CD34", "murmur2":123
                }], "overridePaths":["config/iris"]
            }])
        } else {
            serde_json::json!([])
        };
        z.start_file("gdlpack.json", opts).unwrap();
        z.write_all(
            serde_json::to_string(&serde_json::json!({
                "formatVersion":version, "name":"Fixture", "createdAt":"2024-01-15T10:30:00Z",
                "dependencies":{"minecraft":"1.20.1"}, "entries":entries
            }))
            .unwrap()
            .as_bytes(),
        )
        .unwrap();
        z.start_file("overrides/config/iris/example.json", opts)
            .unwrap();
        z.write_all(content).unwrap();
        z.finish().unwrap();
    }
    fn optional_reason(e: Translation) {
        assert_eq!(
            serde_json::to_value(e).unwrap()["translation"],
            "InstanceImportGdlpackOptionalUnsupported"
        );
    }
    #[test]
    fn task40_preview_refuses_optional_shader_before_admission() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("pack.gdlpack");
        pack(&p, true, 1, b"{}");
        match inspect(&p) {
            Err(e) => optional_reason(e),
            Ok(_) => panic!("unsupported optional shader accepted by production scanner parser"),
        }
    }
    #[tokio::test]
    async fn task40_changed_preview_refuses_before_initializer() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("pack.gdlpack");
        pack(&p, false, 1, b"original");
        assert!(inspect(&p).is_ok());
        pack(&p, true, 1, b"{}");
        let admitted = AdmittedArchive::capture(p, t.path().join("private")).await;
        assert!(
            admitted.is_err(),
            "changed optional archive authorized instance initializer"
        );
        assert!(!t.path().join(".setup").exists());
    }
    #[tokio::test]
    async fn task40_admitted_bytes_survive_external_replacement() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("pack.gdlpack");
        pack(&p, false, 1, b"original");
        let original = std::fs::read(&p).unwrap();
        let admitted = AdmittedArchive::capture(p.clone(), t.path().join("private"))
            .await
            .unwrap();
        pack(&p, true, 1, b"replacement");
        let replacement = std::fs::read(&p).unwrap();
        admitted.install(t.path().join(".setup")).await.unwrap();
        assert_eq!(
            std::fs::read(t.path().join(".setup/gdlpack")).unwrap(),
            original,
            "initializer must install the exact validated archive"
        );
        assert_eq!(
            std::fs::read(p).unwrap(),
            replacement,
            "user archive must remain untouched"
        );
    }
    #[test]
    fn task40_unknown_format_and_required_controls() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("pack.gdlpack");
        pack(&p, false, 2, b"{}");
        assert!(inspect(&p).is_err());
        pack(&p, false, 1, b"{}");
        assert!(inspect(&p).is_ok());
    }
}
