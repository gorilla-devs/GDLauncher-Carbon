use std::{
    env, fs,
    path::{Path, PathBuf},
};

pub(crate) async fn get_runtime_path_override() -> PathBuf {
    #[allow(unused_assignments)]
    let mut path: Option<PathBuf> = None;
    #[cfg(debug_assertions)]
    {
        path = Some(PathBuf::from(env!("RUNTIME_PATH")));
    }
    #[cfg(not(debug_assertions))]
    {
        let mut args = env::args();
        while let Some(arg) = args.next() {
            if arg == "--runtime_path" {
                if let Some(_path) = args.next() {
                    path = Some(PathBuf::from(_path));
                    break;
                }
            }
        }
    }

    let data_path = path.expect("Runtime path not found");

    fs::create_dir_all(&data_path)
        .expect(format!("Failed to create data directory: {:?}", data_path).as_str());

    let resolved = dunce::canonicalize(&data_path)
        .unwrap_or_else(|e| {
            panic!("Failed to canonicalize runtime path {:?}: {}. Please ensure the path exists and is accessible.", data_path, e)
        });
    if cfg!(windows) {
        plain_share_path(&resolved)
    } else {
        resolved
    }
}

/// Longest path Windows programs accept without the `\\?\` form.
const WINDOWS_MAX_PATH: usize = 260;

/// A network share in the form programs expect: `\\?\UNC\server\share\folder` as
/// `\\server\share\folder`. Resolving a folder on a share or on a mapped network drive
/// gives the first form, which dunce simplifies for local drives only and which Java
/// does not take in a classpath. Like dunce for a drive, a path that only the `\\?\`
/// form can name (too long, a reserved device name, a name ending in a dot or space)
/// is returned as it is, and so is every other path.
pub(crate) fn plain_share_path(path: &Path) -> PathBuf {
    let Some(text) = path.to_str() else {
        return path.to_path_buf();
    };
    let Some(rest) = text.strip_prefix(r"\\?\UNC\") else {
        return path.to_path_buf();
    };
    let mut names = rest.split('\\');
    let server_and_share = names
        .by_ref()
        .take(2)
        .filter(|name| !name.is_empty())
        .count()
        == 2;
    if !server_and_share
        || text.len() - r"?\UNC\".len() >= WINDOWS_MAX_PATH
        || rest.contains('/')
        || names.any(needs_verbatim_form)
    {
        return path.to_path_buf();
    }
    PathBuf::from(format!(r"\\{rest}"))
}

/// A folder name the plain path form reads differently from the `\\?\` form.
fn needs_verbatim_form(name: &str) -> bool {
    const RESERVED: [&str; 22] = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    let stem = name.split('.').next().unwrap_or(name).trim_end();
    name.is_empty()
        || name == "."
        || name == ".."
        || name.ends_with(['.', ' '])
        || RESERVED
            .iter()
            .any(|reserved| stem.eq_ignore_ascii_case(reserved))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(path: &str) -> String {
        plain_share_path(Path::new(path))
            .to_str()
            .unwrap()
            .to_owned()
    }

    #[test]
    fn a_resolved_share_is_given_in_the_plain_form() {
        assert_eq!(
            plain(r"\\?\UNC\fileserver\home$\steve\gdlauncher_carbon\data"),
            r"\\fileserver\home$\steve\gdlauncher_carbon\data"
        );
        assert_eq!(plain(r"\\?\UNC\nas\games"), r"\\nas\games");
        assert_eq!(
            plain(r"\\?\UNC\10.0.0.2\Jeux Vidéo\GDL (new)"),
            r"\\10.0.0.2\Jeux Vidéo\GDL (new)"
        );
        assert_eq!(
            plain(r"\\?\UNC\nas\games\.minecraft\v1.2"),
            r"\\nas\games\.minecraft\v1.2"
        );
    }

    /// Every install that is not on a share keeps the path it has today.
    #[test]
    fn every_other_path_is_left_as_it_is() {
        for path in [
            r"C:\Users\Steve\AppData\Roaming\gdlauncher_carbon\data",
            r"D:\Games\GDLauncher (new)\data",
            r"E:\",
            r"\\?\C:\Users\Steve\AppData\Roaming\gdlauncher_carbon\data",
            r"\\?\Volume{b75e2c83-0000-0000-0000-602f00000000}\data",
            r"\\.\pipe\data",
            r"\\fileserver\home$\steve\data",
            r"\\?\unc\nas\games",
            "/home/steve/.local/share/gdlauncher_carbon/data",
            "/Users/Alex Doe/Library/Application Support/gdlauncher_carbon/data",
            "/mnt/UNC/data",
            "relative/data",
            "",
        ] {
            assert_eq!(plain(path), path);
        }
    }

    /// The paths only the long form can name stay in it, as dunce does for a drive.
    #[test]
    fn a_share_path_only_the_long_form_can_name_is_left_as_it_is() {
        let too_long = format!(r"\\?\UNC\nas\games\{}", "a".repeat(250));
        for path in [
            too_long.as_str(),
            r"\\?\UNC\nas\games\con",
            r"\\?\UNC\nas\games\NUL.txt\data",
            r"\\?\UNC\nas\games\LPT1",
            r"\\?\UNC\nas\games\ends with dot.",
            r"\\?\UNC\nas\games\ends with space \data",
            r"\\?\UNC\nas\games\..\data",
            r"\\?\UNC\nas\games\a/b",
            r"\\?\UNC\nas\games\",
            r"\\?\UNC\nas\games\\data",
            r"\\?\UNC\nas",
            r"\\?\UNC\nas\",
            r"\\?\UNC\\games",
            r"\\?\UNC\",
        ] {
            assert_eq!(plain(path), path);
        }
        let longest = format!(
            r"\\?\UNC\nas\games\{}",
            "a".repeat(259 - r"\\nas\games\".len())
        );
        assert_eq!(plain(&longest).len(), 259);
        assert_eq!(plain(&format!("{longest}a")), format!("{longest}a"));
    }

    #[test]
    fn the_plain_form_is_stable() {
        let once = plain(r"\\?\UNC\nas\games\data");
        assert_eq!(plain(&once), once);
    }
}
