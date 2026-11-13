//! The launch arguments Java receives. Filling in a version file's placeholders used to
//! un-escape the filled-in result, which broke every path that starts with two
//! backslashes; these tests pin that values now arrive untouched and that nothing else
//! about the arguments changed for installs that worked before.

use super::*;
use rand::{Rng, SeedableRng, rngs::StdRng};

/// Every argument template the metadata server (meta.gdl.gg, format v2) served on
/// 2026-11-13: all 923 Minecraft versions and every Forge (4531), NeoForge (1916),
/// Fabric (255) and Quilt (312) loader version, `arguments.jvm`, `arguments.game` and
/// the words of `minecraftArguments`, deduplicated.
const SERVED_TEMPLATES: &str = include_str!("../../../fixtures/launch_argument_templates.txt");

/// The substitution exactly as it was before the change, kept to compare against.
fn substitute_as_before(
    regex: &Regex,
    replacer_args: &ReplacerArgs,
    ignore_list_jar: &str,
    argument: &str,
) -> String {
    let mut argument = argument.to_string();
    if argument.starts_with("-DignoreList=") {
        argument.push_str(&format!(",{ignore_list_jar}.jar"));
    }

    regex
        .replace_all(&argument, |caps: &Captures| {
            if let Some(value) = caps.name("value") {
                let value = match value.as_str().try_into() {
                    Ok(value) => replace_placeholder(replacer_args, value),
                    Err(_) => return String::new(),
                };
                return format!("--{} {}", caps.name("arg").unwrap().as_str(), value);
            } else if let Some(standalone) = caps.name("standalone") {
                return match standalone.as_str().try_into() {
                    Ok(standalone) => replace_placeholder(replacer_args, standalone),
                    Err(_) => String::new(),
                };
            }
            caps.name("arg").unwrap().as_str().to_string()
        })
        .replace("\\\"", "\"")
        .replace("\\\\", "\\")
}

/// The values of one install: its data folder, the separator its paths use and the
/// separator of its classpath.
fn install(root: &str, sep: char, classpath_separator: char, player: &str) -> ReplacerArgs {
    let under = |parts: &[&str]| {
        let mut path = root.to_string();
        for part in parts {
            path.push(sep);
            path.push_str(part);
        }
        path
    };
    let libraries = [
        &[
            "libraries",
            "com",
            "mojang",
            "brigadier",
            "1.3.10",
            "brigadier-1.3.10.jar",
        ][..],
        &[
            "libraries",
            "org",
            "lwjgl",
            "lwjgl",
            "3.3.3",
            "lwjgl-3.3.3-natives-windows.jar",
        ],
        &[
            "libraries",
            "net",
            "minecraft",
            "client",
            "1.21.1",
            "client-1.21.1.jar",
        ],
    ]
    .iter()
    .map(|parts| under(parts))
    .collect::<Vec<_>>()
    .join(&classpath_separator.to_string());
    ReplacerArgs {
        player_name: player.to_owned(),
        player_token: "eyJraWQiOiJhYzg0YSIsImFsZyI6IkhTMjU2In0.eyJ4dWlkIjoiMjUzNTQ.c2lnbmF0dXJl-_"
            .to_owned(),
        version_name: "1.21.1".to_owned(),
        game_directory: InstancePath::new(PathBuf::from(under(&["instances", "My Pack (2)"]))),
        game_assets: PathBuf::from(under(&["assets", "virtual", "legacy"])),
        library_directory: PathBuf::from(under(&["libraries"])),
        natives_path: PathBuf::from(under(&["natives", "1.21.1"])),
        assets_root: PathBuf::from(under(&["assets"])),
        assets_index_name: "17".to_owned(),
        auth_uuid: "069a79f444e94726a5befca90e38aaf5".to_owned(),
        libraries,
        auth_access_token:
            "eyJraWQiOiJhYzg0YSIsImFsZyI6IkhTMjU2In0.eyJ4dWlkIjoiMjUzNTQ.c2lnbmF0dXJl-_".to_owned(),
        auth_session: "token:eyJraWQiOiJhYzg0YSJ9:069a79f444e94726a5befca90e38aaf5".to_owned(),
        user_type: "msa".to_owned(),
        version_type: "release".to_owned(),
        user_properties: "{}".to_owned(),
    }
}

/// Data folders of installs that launch today, on each system.
fn working_installs() -> Vec<(&'static str, ReplacerArgs)> {
    vec![
        (
            "linux",
            install(
                "/home/steve/.local/share/gdlauncher_carbon/data",
                '/',
                ':',
                "Steve",
            ),
        ),
        (
            "linux, spaces, quotes and accents",
            install(
                "/mnt/Jeux Vidéo/it's \"mine\"/gdl data",
                '/',
                ':',
                "xX_Steve_Xx",
            ),
        ),
        (
            "linux flatpak",
            install(
                "/home/steve/.var/app/io.gdlauncher/data/gdlauncher_carbon/data",
                '/',
                ':',
                "S",
            ),
        ),
        (
            "macos",
            install(
                "/Users/Alex Doe/Library/Application Support/gdlauncher_carbon/data",
                '/',
                ':',
                "Alex",
            ),
        ),
        (
            "windows",
            install(
                r"C:\Users\Steve\AppData\Roaming\gdlauncher_carbon\data",
                '\\',
                ';',
                "Steve",
            ),
        ),
        (
            "windows, spaces and accents",
            install(
                r"C:\Users\José Müller\AppData\Roaming\gdlauncher_carbon\data",
                '\\',
                ';',
                "Jose_M",
            ),
        ),
        (
            "windows, other drive",
            install(r"D:\Games\GDLauncher (new)\data", '\\', ';', "Steve"),
        ),
        (
            "windows, drive root",
            install(r"E:\gdl", '\\', ';', "Steve"),
        ),
        (
            "windows, cyrillic and cjk",
            install(r"C:\Users\Алексей\Игры\启动器\data", '\\', ';', "Alexey"),
        ),
        (
            "windows, onedrive folder",
            install(
                r"C:\Users\Steve\OneDrive - Company\Documents\GDL",
                '\\',
                ';',
                "Steve",
            ),
        ),
    ]
}

fn templates() -> Vec<&'static str> {
    SERVED_TEMPLATES.lines().collect()
}

#[test]
fn the_fixture_holds_every_served_template() {
    let templates = templates();
    assert!(templates.len() > 4000, "{}", templates.len());
    for expected in [
        "${classpath}",
        "-Djava.library.path=${natives_directory}",
        "-DlibraryDirectory=${library_directory}",
        "--gameDir",
        "${game_directory}",
        "--assetsDir",
        "${assets_root}",
        "${game_assets}",
        "${auth_session}",
        "${user_properties}",
    ] {
        assert!(templates.contains(&expected), "{expected}");
    }
}

/// The un-escape only ever had the filled-in values to act on: no version file the
/// server has ever served contains a backslash or a quote.
#[test]
fn no_served_template_has_anything_to_unescape() {
    for template in templates() {
        assert!(
            !template.contains('\\') && !template.contains('"'),
            "{template}"
        );
    }
}

#[test]
fn every_served_template_gives_working_installs_the_arguments_they_had() {
    let regex = launch_argument_regex();
    let mut compared = 0;
    for (name, values) in working_installs() {
        for template in templates() {
            for jar in ["1.21.1", "1.7.10-Forge10.13.4.1614-1.7.10", "24w14potato"] {
                assert_eq!(
                    substitute_launch_argument(&regex, &values, jar, template),
                    substitute_as_before(&regex, &values, jar, template),
                    "{name}: {template}"
                );
                compared += 1;
            }
        }
    }
    assert!(compared > 120_000, "{compared}");
}

/// The same comparison over made-up templates that do contain escapes, spaces, `--name
/// ${value}` pairs and several placeholders, for the version files not served yet.
#[test]
fn made_up_templates_with_escapes_give_working_installs_the_arguments_they_had() {
    let regex = launch_argument_regex();
    let pieces = [
        "\\\\",
        "\\\"",
        "\\",
        "\"",
        " ",
        "=",
        "-D",
        "--",
        "a",
        "path",
        ",",
        "/",
        ":",
        ";",
        "${classpath}",
        "${natives_directory}",
        "${game_directory}",
        "${assets_root}",
        "${game_assets}",
        "${library_directory}",
        "${classpath_separator}",
        "${auth_player_name}",
        "${version_name}",
        "${auth_access_token}",
        "${user_properties}",
        "${launcher_name}",
        "--gameDir ${game_directory}",
        "--username ${auth_player_name}",
        "-DignoreList=",
    ];
    let installs = working_installs();
    let mut rng = StdRng::seed_from_u64(0x6d1a);
    for round in 0..20_000 {
        let template: String = (0..rng.gen_range(1..9))
            .map(|_| pieces[rng.gen_range(0..pieces.len())])
            .collect();
        let (name, values) = &installs[round % installs.len()];
        assert_eq!(
            substitute_launch_argument(&regex, values, "1.21.1", &template),
            substitute_as_before(&regex, values, "1.21.1", &template),
            "{name}: {template:?}"
        );
    }
}

#[test]
fn the_escapes_of_a_version_file_are_still_undone() {
    let regex = launch_argument_regex();
    let values = install("/data", '/', ':', "Steve");
    let substitute =
        |template: &str| substitute_launch_argument(&regex, &values, "1.21.1", template);
    assert_eq!(
        substitute(r#"-Dname=\"two words\""#),
        r#"-Dname="two words""#
    );
    assert_eq!(
        substitute(r"-Dpath=C:\\Program Files\\x"),
        r"-Dpath=C:\Program Files\x"
    );
    assert_eq!(substitute(r#"-Dmixed=\\\"x"#), r#"-Dmixed=\"x"#);
    assert_eq!(
        substitute(r#"-Dname=\"${auth_player_name}\""#),
        r#"-Dname="Steve""#
    );
}

#[test]
fn every_form_of_argument_is_filled_in() {
    let regex = launch_argument_regex();
    let values = install("/data", '/', ':', "Steve");
    let substitute =
        |template: &str| substitute_launch_argument(&regex, &values, "1.20.1", template);
    assert_eq!(substitute("--username"), "--username");
    assert_eq!(substitute("${auth_player_name}"), "Steve");
    assert_eq!(
        substitute("--username ${auth_player_name}"),
        "--username Steve"
    );
    assert_eq!(
        substitute("-Djava.library.path=${natives_directory}"),
        "-Djava.library.path=/data/natives/1.21.1"
    );
    assert_eq!(
        substitute("${library_directory}/a.jar${classpath_separator}${library_directory}/b.jar"),
        format!("/data/libraries/a.jar{CLASSPATH_SEPARATOR}/data/libraries/b.jar")
    );
    assert_eq!(
        substitute("-DignoreList=bootstraplauncher,client-extra"),
        "-DignoreList=bootstraplauncher,client-extra,1.20.1.jar"
    );
    assert_eq!(substitute("${not_a_placeholder}"), "");
    assert_eq!(
        substitute("-Dminecraft.launcher.brand=${launcher_name}"),
        "-Dminecraft.launcher.brand=GDLauncher"
    );
}

/// The data folders no game could start from.
fn share_installs() -> Vec<(&'static str, ReplacerArgs)> {
    vec![
        (
            "share",
            install(
                r"\\fileserver\home$\steve\gdlauncher_carbon\data",
                '\\',
                ';',
                "Steve",
            ),
        ),
        (
            "mapped drive, as Windows resolves it",
            install(
                r"\\?\UNC\fileserver\home$\steve\gdlauncher_carbon\data",
                '\\',
                ';',
                "Steve",
            ),
        ),
        (
            "path too long to simplify",
            install(
                r"\\?\C:\Users\Steve\AppData\Roaming\gdlauncher_carbon\data",
                '\\',
                ';',
                "Steve",
            ),
        ),
    ]
}

#[test]
fn a_value_reaches_java_exactly_as_it_is() {
    let regex = launch_argument_regex();
    for (name, values) in working_installs().into_iter().chain(share_installs()) {
        let substitute =
            |template: &str| substitute_launch_argument(&regex, &values, "1.21.1", template);
        let game_directory = values.game_directory.get_data_path().display().to_string();
        let natives = values.natives_path.display().to_string();
        let assets = values.assets_root.display().to_string();
        let game_assets = values.game_assets.display().to_string();
        let library_directory = values.library_directory.display().to_string();
        assert_eq!(substitute("${classpath}"), values.libraries, "{name}");
        assert_eq!(substitute("${game_directory}"), game_directory, "{name}");
        assert_eq!(
            substitute("--gameDir ${game_directory}"),
            format!("--gameDir {game_directory}"),
            "{name}"
        );
        assert_eq!(substitute("${assets_root}"), assets, "{name}");
        assert_eq!(substitute("${game_assets}"), game_assets, "{name}");
        assert_eq!(
            substitute("-Djava.library.path=${natives_directory}"),
            format!("-Djava.library.path={natives}"),
            "{name}"
        );
        assert_eq!(
            substitute("-DlibraryDirectory=${library_directory}"),
            format!("-DlibraryDirectory={library_directory}"),
            "{name}"
        );
        assert_eq!(
            substitute("${auth_access_token}"),
            values.auth_access_token,
            "{name}"
        );
        assert_eq!(substitute("${auth_session}"), values.auth_session, "{name}");
        assert_eq!(
            substitute("${auth_player_name}"),
            values.player_name,
            "{name}"
        );
    }
}

/// What the change is for, and the only installs whose arguments differ from before.
#[test]
fn a_share_path_keeps_its_leading_backslashes() {
    let regex = launch_argument_regex();
    for (name, values) in share_installs() {
        for template in [
            "${classpath}",
            "${game_directory}",
            "-Djava.library.path=${natives_directory}",
        ] {
            let now = substitute_launch_argument(&regex, &values, "1.21.1", template);
            let before = substitute_as_before(&regex, &values, "1.21.1", template);
            assert!(now.contains(r"\\"), "{name}: {now}");
            assert!(!before.contains(r"\\"), "{name}: {before}");
        }
    }
}
