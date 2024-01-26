use std::{
    fs::{self, DirEntry},
    io,
    path::Path,
};

use anyhow::bail;

/// Less featureful version of WalkDir that exposes relative paths with forward slashes
/// as separators regardless of platform. It also requires UTF-8 filenames.
pub struct NormalizedWalkdir {
    parts: Vec<(fs::ReadDir, usize)>,
    path: String,
}

pub struct NormalizedDirEntry<'a> {
    pub name: &'a str,
    pub relative_path: &'a str,
    pub entry: DirEntry,
    pub is_dir: bool,
}

impl NormalizedWalkdir {
    pub fn new(path: &Path) -> io::Result<Self> {
        Ok(Self {
            parts: vec![(fs::read_dir(path)?, 0)],
            path: String::new(),
        })
    }

    fn last_dir_end(&self) -> usize {
        self.parts
            .iter()
            .next_back()
            .map(|(_, start)| *start)
            .unwrap_or(0)
    }

    pub fn next(&mut self) -> anyhow::Result<Option<NormalizedDirEntry>> {
        while let Some((reader, part_start)) = self.parts.last_mut() {
            if let Some(entry) = reader.next() {
                let entry = entry?;
                let path = entry.path();
                let name = path
                    .file_name()
                    .expect("the path cannot be terminated in ..");

                let Some(name) = name.to_str() else {
                    bail!(
                        "encountered non UTF-8 filename at '{}'",
                        entry.path().to_string_lossy()
                    );
                };

                self.path.truncate(self.last_dir_end());

                let metadata = fs::metadata(&path)?;
                let name_from = self.path.len() + 1;

                self.path.reserve(name.len() + 1);
                self.path.push('/');
                self.path.push_str(name);

                let is_dir = metadata.is_dir();
                if is_dir {
                    let reader = fs::read_dir(&path)?;
                    self.parts.push((reader, self.path.len()));
                }

                return Ok(Some(NormalizedDirEntry {
                    entry,
                    relative_path: &self.path,
                    name: &self.path[name_from..],
                    is_dir,
                }));
            } else {
                self.parts.pop();
            }
        }

        Ok(None)
    }
}
