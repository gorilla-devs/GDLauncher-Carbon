use carbon_platforms::{ModChannel, ModPlatform, ModSources, RemoteVersion};
use chrono::{DateTime, Utc};

fn same_version(a: &RemoteVersion, b: &RemoteVersion) -> bool {
    match (a, b) {
        (RemoteVersion::Curseforge(a), RemoteVersion::Curseforge(b)) => a.id == b.id,
        (RemoteVersion::Modrinth(a), RemoteVersion::Modrinth(b)) => a.id == b.id,
        _ => false,
    }
}

pub(crate) fn strictly_newer<T: Ord>(
    same_project: bool,
    same_id: bool,
    candidate_date: &T,
    installed_date: &T,
) -> bool {
    same_project && !same_id && candidate_date > installed_date
}

pub(crate) fn newer(candidate: &RemoteVersion, installed: &RemoteVersion) -> bool {
    let same_project = match (candidate, installed) {
        (RemoteVersion::Curseforge(a), RemoteVersion::Curseforge(b)) => a.mod_id == b.mod_id,
        (RemoteVersion::Modrinth(a), RemoteVersion::Modrinth(b)) => a.project_id == b.project_id,
        _ => false,
    };
    strictly_newer(
        same_project,
        same_version(candidate, installed),
        &candidate.date(),
        &installed.date(),
    )
}

/// The least stable channel an update of a file installed on `installed` may
/// move it to.
///
/// An update never leaves the installed file less stable than it is, unless
/// the policy itself prefers a less stable channel (a beta-first list, or
/// stable updates turned off): the first channel that allows updates is the one
/// the user asked for, so it lowers the floor. This is the Modrinth App's
/// `least_stable(preferred, installed)`. With the shipped default (stable,
/// beta, alpha, all allowed) it is simply the installed channel, so a Release
/// is only ever replaced by a newer Release, as earlier releases of the
/// launcher and the CurseForge app do. `None` when no channel allows updates.
pub(crate) fn channel_floor(installed: ModChannel, sources: &ModSources) -> Option<ModChannel> {
    sources
        .channels
        .iter()
        .find(|channel| channel.allow_updates)
        .map(|preferred| installed.min(preferred.channel))
}

/// The channel an installed jar is held to, from the label each platform's
/// metadata cache recorded for it (`None` for a platform that does not list it
/// or whose label is unreadable).
///
/// One jar can carry a different label on each platform, for example Release
/// on CurseForge and Beta on Modrinth. The most stable label wins, and both
/// platforms' badges and candidates use it, so a file either platform calls a
/// Release is only replaced by a newer Release.
pub(crate) fn installed_channel(
    labels: impl IntoIterator<Item = Option<ModChannel>>,
) -> Option<ModChannel> {
    labels.into_iter().flatten().max()
}

/// The platform identity of an installed file.
#[derive(Clone, Debug)]
pub(crate) enum InstalledId {
    Curseforge {
        project_id: i32,
        file_id: i32,
    },
    Modrinth {
        project_id: String,
        version_id: String,
    },
}

impl InstalledId {
    /// Any other file of the same project.
    fn other_file_of_project(&self, candidate: &RemoteVersion) -> bool {
        match (self, candidate) {
            (
                Self::Curseforge {
                    project_id,
                    file_id,
                },
                RemoteVersion::Curseforge(file),
            ) => file.mod_id == *project_id && file.id != *file_id,
            (
                Self::Modrinth {
                    project_id,
                    version_id,
                },
                RemoteVersion::Modrinth(version),
            ) => version.project_id == *project_id && version.id != *version_id,
            _ => false,
        }
    }
}

/// What one platform says about the file an addon has installed.
///
/// `channel` is the installed jar's cached channel (see `installed_channel`),
/// which is also what the update badge reads, so the badge and the resolver
/// apply the same floor.
pub(crate) enum Installed {
    /// The platform still serves the installed file: only files published
    /// after it are updates.
    Present {
        version: RemoteVersion,
        channel: ModChannel,
    },
    /// The platform no longer serves the installed file (its author or the
    /// platform's moderation removed it), so the update that replaces a pulled
    /// build still works. On CurseForge, which numbers files in upload order,
    /// the files with a higher id replace it. On Modrinth, if the other
    /// platform still lists the same jar, only files published after that
    /// listing replace it; otherwise any other version of the project can,
    /// and `choose_update` takes the newest one at or above the channel floor.
    Gone {
        id: InstalledId,
        channel: ModChannel,
    },
}

impl Installed {
    /// The installed file on its live channel.
    #[cfg(test)]
    pub fn present(version: RemoteVersion) -> Self {
        let channel = version.channel();
        Self::Present { version, channel }
    }

    fn channel(&self) -> ModChannel {
        match self {
            Self::Present { channel, .. } | Self::Gone { channel, .. } => *channel,
        }
    }

    /// When the installed file was published, if its platform still serves it.
    fn published(&self) -> Option<DateTime<Utc>> {
        match self {
            Self::Present { version, .. } => Some(version.date()),
            Self::Gone { .. } => None,
        }
    }

    /// `listed`: when a platform that still serves the installed jar
    /// published it.
    fn replaced_by(
        &self,
        candidate: &RemoteVersion,
        sources: &ModSources,
        listed: Option<DateTime<Utc>>,
    ) -> bool {
        let follows = match (self, candidate) {
            (Self::Present { version, .. }, _) => newer(candidate, version),
            // CurseForge's own upload order, not the other platform's date:
            // a listing there may be later than the files that followed.
            (
                Self::Gone {
                    id:
                        InstalledId::Curseforge {
                            project_id,
                            file_id,
                        },
                    ..
                },
                RemoteVersion::Curseforge(file),
            ) => file.mod_id == *project_id && file.id > *file_id,
            (Self::Gone { id, .. }, _) => {
                id.other_file_of_project(candidate)
                    && listed.is_none_or(|listed| candidate.date() > listed)
            }
        };
        follows
            && channel_floor(self.channel(), sources)
                .is_some_and(|floor| candidate.channel() >= floor)
    }
}

pub(crate) fn choose_update(
    mut versions: Vec<RemoteVersion>,
    installed: &[Installed],
    sources: &ModSources,
) -> Option<RemoteVersion> {
    let listed = installed.iter().filter_map(Installed::published).max();
    versions.retain(|candidate| {
        let platform = match candidate {
            RemoteVersion::Curseforge(_) => ModPlatform::Curseforge,
            RemoteVersion::Modrinth(_) => ModPlatform::Modrinth,
        };
        !sources.platform_blacklist.contains(&platform)
            && installed
                .iter()
                .any(|current| current.replaced_by(candidate, sources, listed))
    });
    versions.sort();
    let unordered = listed.is_none()
        && installed.iter().any(|current| {
            matches!(
                current,
                Installed::Gone {
                    id: InstalledId::Modrinth { .. },
                    ..
                }
            )
        });
    if unordered {
        // Modrinth no longer serves the installed version and no platform
        // lists the jar, so nothing tells which versions came after it.
        // Walking stable first here would trade a removed beta for an older
        // Release from an earlier line, so take the newest file at or above
        // the floor instead (as the Modrinth App does): a removed Release gets
        // the newest Release, and a removed beta the newest beta, or a Release
        // only if that Release is newer still.
        return versions.into_iter().find(|candidate| {
            sources
                .channels
                .iter()
                .any(|channel| channel.allow_updates && channel.channel == candidate.channel())
        });
    }
    for channel in &sources.channels {
        if channel.allow_updates {
            if let Some(index) = versions.iter().position(|v| v.channel() == channel.channel) {
                return Some(versions.remove(index));
            }
        }
    }
    None
}

/// All policy-relevant target state. Channel preferences are applied on read;
/// all channels are cached, including negative results, independently per context.
#[derive(Clone, Debug)]
pub(crate) struct UpdateContext {
    pub game: String,
    pub loaders: Vec<String>,
    pub shader_engines: Vec<String>,
}

impl UpdateContext {
    pub fn base_key(&self) -> String {
        let mut loaders = self.loaders.clone();
        loaders.sort();
        loaders.dedup();
        let mut engines = self.shader_engines.clone();
        engines.sort();
        engines.dedup();
        serde_json::to_string(&(&self.game, loaders, engines)).expect("string context")
    }
    pub fn key(&self, kind: &str) -> String {
        format!("{}:{kind}", self.base_key())
    }
    pub fn request_loaders(&self, kind: &str) -> Vec<String> {
        let mut loaders = self.loaders.clone();
        match kind {
            "resourcepacks" => loaders.push("minecraft".into()),
            "datapacks" => loaders.extend(["minecraft".into(), "datapack".into()]),
            "shaders" => loaders = self.shader_engines.clone(),
            _ => {}
        }
        loaders.sort();
        loaders.dedup();
        loaders
    }
    pub fn compatible(&self, kind: &str, games: &[String], loaders: &[String]) -> bool {
        games.iter().any(|g| g.eq_ignore_ascii_case(&self.game))
            && loaders.iter().any(|l| {
                self.request_loaders(kind)
                    .iter()
                    .any(|want| want.eq_ignore_ascii_case(l))
            })
    }
    pub fn cf_compatible(&self, kind: &str, games_and_loaders: &[String]) -> bool {
        games_and_loaders
            .iter()
            .any(|g| g.eq_ignore_ascii_case(&self.game))
            && match kind {
                "mods" => games_and_loaders
                    .iter()
                    .any(|l| self.loaders.iter().any(|want| want.eq_ignore_ascii_case(l))),
                "shaders" => !self.shader_engines.is_empty(),
                "resourcepacks" | "datapacks" => true,
                _ => false,
            }
    }
    /// Whether a cached `gamever,loader,channel` path list holds an update the
    /// policy would install over a file on the `installed` channel.
    ///
    /// Applied at read time, so rows cached before the channel floor existed
    /// (with paths for every newer channel) are judged by the same rule as
    /// `choose_update` without waiting for a refresh.
    pub fn has_path(
        &self,
        kind: &str,
        paths: &str,
        sources: &ModSources,
        installed: ModChannel,
    ) -> bool {
        let Some(floor) = channel_floor(installed, sources) else {
            return false;
        };
        paths.split(';').any(|path| {
            let parts: Vec<_> = path.split(',').collect();
            parts.len() == 3
                && parts[0].eq_ignore_ascii_case(&self.game)
                && (parts[1] == kind
                    && (matches!(kind, "resourcepacks" | "datapacks")
                        || kind == "shaders" && !self.shader_engines.is_empty())
                    || self.request_loaders(kind).iter().any(|l| l == parts[1]))
                && sources.channels.iter().any(|c| {
                    c.allow_updates && c.channel >= floor && c.channel.as_str() == parts[2]
                })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use carbon_platforms::{ModChannel, ModChannelWithUsage};
    fn version(id: &str, day: u32, channel: &str) -> RemoteVersion {
        RemoteVersion::Modrinth(
            serde_json::from_value(serde_json::json!({
                "id":id, "project_id":"project", "name":id, "version_number":id,
                "dependencies":[], "game_versions":["1.20.1"], "version_type":channel,
                "loaders":["fabric"], "featured":false, "author_id":"author",
                "date_published":format!("2026-01-{day:02}T00:00:00Z"), "downloads":0, "files":[]
            }))
            .unwrap(),
        )
    }
    fn sources() -> ModSources {
        policy(&[
            (ModChannel::Stable, true),
            (ModChannel::Beta, true),
            (ModChannel::Alpha, true),
        ])
    }
    fn policy(channels: &[(ModChannel, bool)]) -> ModSources {
        ModSources {
            channels: channels
                .iter()
                .map(|&(channel, allow_updates)| ModChannelWithUsage {
                    channel,
                    allow_updates,
                })
                .collect(),
            platform_blacklist: vec![],
        }
    }
    /// A CurseForge file of project 42. `release_type`: 1 release, 2 beta, 3 alpha.
    fn cf_file(id: i32, day: u32, release_type: u8) -> RemoteVersion {
        RemoteVersion::Curseforge(
            serde_json::from_value(serde_json::json!({
                "id": id, "gameId": 432, "modId": 42, "isAvailable": true,
                "displayName": format!("file-{id}"), "fileName": format!("file-{id}.jar"),
                "releaseType": release_type, "fileStatus": 4, "hashes": [],
                "fileDate": format!("2026-01-{day:02}T00:00:00Z"), "fileLength": 1,
                "downloadCount": 0, "downloadUrl": null, "gameVersions": ["1.20.1", "Fabric"],
                "sortableGameVersions": [], "dependencies": [], "fileFingerprint": 0
            }))
            .unwrap(),
        )
    }
    fn gone(channel: ModChannel) -> Installed {
        Installed::Gone {
            id: InstalledId::Modrinth {
                project_id: "project".into(),
                version_id: "removed".into(),
            },
            channel,
        }
    }
    fn chosen_id(chosen: Option<RemoteVersion>) -> Option<String> {
        chosen.map(|chosen| match chosen {
            RemoteVersion::Modrinth(v) => v.id,
            RemoteVersion::Curseforge(f) => f.id.to_string(),
        })
    }
    fn fabric() -> UpdateContext {
        UpdateContext {
            game: "1.20.1".into(),
            loaders: vec!["fabric".into()],
            shader_engines: vec![],
        }
    }

    #[test]
    fn default_floor_is_the_installed_channel() {
        for installed in [ModChannel::Stable, ModChannel::Beta, ModChannel::Alpha] {
            assert_eq!(channel_floor(installed, &sources()), Some(installed));
        }
        assert_eq!(
            channel_floor(
                ModChannel::Stable,
                &policy(&[
                    (ModChannel::Stable, false),
                    (ModChannel::Beta, false),
                    (ModChannel::Alpha, false),
                ])
            ),
            None,
            "no channel allows updates"
        );
    }

    #[test]
    fn task11_channel_resolver_keeps_installed_release_on_stable() {
        let installed = [Installed::present(version("installed", 10, "release"))];
        let prereleases = || vec![version("beta", 20, "beta"), version("alpha", 25, "alpha")];
        assert_eq!(
            chosen_id(choose_update(prereleases(), &installed, &sources())),
            None,
            "a Release on its latest stable build has no update under the default policy"
        );
        let mut with_release = prereleases();
        with_release.push(version("stable", 15, "release"));
        assert_eq!(
            chosen_id(choose_update(with_release, &installed, &sources())).as_deref(),
            Some("stable"),
            "a newer Release still updates a Release"
        );
    }

    #[test]
    fn installed_beta_moves_to_newer_stable_or_beta_but_never_alpha() {
        let installed = [Installed::present(version("installed", 10, "beta"))];
        assert_eq!(
            chosen_id(choose_update(
                vec![
                    version("stable", 20, "release"),
                    version("beta", 25, "beta")
                ],
                &installed,
                &sources()
            ))
            .as_deref(),
            Some("stable"),
            "a newer stable is preferred"
        );
        assert_eq!(
            chosen_id(choose_update(
                vec![version("beta", 25, "beta"), version("alpha", 28, "alpha")],
                &installed,
                &sources()
            ))
            .as_deref(),
            Some("beta")
        );
        assert_eq!(
            chosen_id(choose_update(
                vec![version("alpha", 28, "alpha")],
                &installed,
                &sources()
            )),
            None,
            "an installed beta never moves to an alpha"
        );
    }

    #[test]
    fn installed_alpha_moves_to_newer_alpha() {
        assert_eq!(
            chosen_id(choose_update(
                vec![version("alpha", 20, "alpha")],
                &[Installed::present(version("installed", 10, "alpha"))],
                &sources()
            ))
            .as_deref(),
            Some("alpha")
        );
    }

    #[test]
    fn task11_channel_resolver_never_downgrades_installed_beta() {
        assert!(
            choose_update(
                vec![
                    version("old-stable", 5, "release"),
                    version("beta", 20, "beta")
                ],
                &[Installed::present(version("beta", 20, "beta"))],
                &sources()
            )
            .is_none()
        );
    }

    #[test]
    fn badge_paths_respect_the_installed_channel() {
        let context = fabric();
        let prereleases = "1.20.1,fabric,beta;1.20.1,fabric,alpha";
        assert!(
            !context.has_path("mods", prereleases, &sources(), ModChannel::Stable),
            "a Release is not badged for betas or alphas"
        );
        assert!(context.has_path(
            "mods",
            &format!("{prereleases};1.20.1,fabric,stable"),
            &sources(),
            ModChannel::Stable
        ));
        assert!(context.has_path("mods", "1.20.1,fabric,beta", &sources(), ModChannel::Beta));
        assert!(!context.has_path("mods", "1.20.1,fabric,alpha", &sources(), ModChannel::Beta));
        assert!(context.has_path("mods", "1.20.1,fabric,alpha", &sources(), ModChannel::Alpha));
    }

    #[test]
    fn an_explicit_prerelease_policy_still_opts_releases_into_betas() {
        let installed = [Installed::present(version("installed", 10, "release"))];
        let beta_first = policy(&[
            (ModChannel::Beta, true),
            (ModChannel::Stable, true),
            (ModChannel::Alpha, true),
        ]);
        let stable_disabled = policy(&[
            (ModChannel::Stable, false),
            (ModChannel::Beta, true),
            (ModChannel::Alpha, true),
        ]);
        for policy in [beta_first, stable_disabled] {
            assert_eq!(
                chosen_id(choose_update(
                    vec![
                        version("stable", 20, "release"),
                        version("beta", 25, "beta")
                    ],
                    &installed,
                    &policy
                ))
                .as_deref(),
                Some("beta"),
                "{policy:?}"
            );
            assert!(
                fabric().has_path("mods", "1.20.1,fabric,beta", &policy, ModChannel::Stable),
                "{policy:?}"
            );
            assert_eq!(
                chosen_id(choose_update(
                    vec![version("alpha", 30, "alpha")],
                    &installed,
                    &policy
                )),
                None,
                "a beta preference does not reach alphas: {policy:?}"
            );
            assert!(
                !fabric().has_path("mods", "1.20.1,fabric,alpha", &policy, ModChannel::Stable),
                "{policy:?}"
            );
        }
    }

    #[test]
    fn a_removed_installed_file_is_replaced_by_any_file_of_its_project() {
        let gone = |channel| [gone(channel)];
        assert_eq!(
            chosen_id(choose_update(
                vec![version("stable", 5, "release"), version("beta", 20, "beta")],
                &gone(ModChannel::Stable),
                &sources()
            ))
            .as_deref(),
            Some("stable"),
            "the Release replaces the removed file even though it predates it"
        );
        assert_eq!(
            chosen_id(choose_update(
                vec![version("beta", 20, "beta")],
                &gone(ModChannel::Stable),
                &sources()
            )),
            None,
            "the channel floor still applies to a removed Release"
        );
        assert_eq!(
            chosen_id(choose_update(
                vec![version("beta", 20, "beta")],
                &gone(ModChannel::Beta),
                &sources()
            ))
            .as_deref(),
            Some("beta")
        );
        let mut foreign = version("foreign", 30, "release");
        if let RemoteVersion::Modrinth(v) = &mut foreign {
            v.project_id = "other".into();
        }
        assert_eq!(
            chosen_id(choose_update(
                vec![foreign, version("removed", 25, "release")],
                &gone(ModChannel::Stable),
                &sources()
            )),
            None,
            "neither another project nor the removed file itself replaces it"
        );
    }

    #[test]
    fn a_removed_file_is_replaced_by_the_newest_file_at_or_above_its_channel() {
        let pick = |channel, candidates| {
            chosen_id(choose_update(candidates, &[gone(channel)], &sources()))
        };
        assert_eq!(
            pick(
                ModChannel::Beta,
                vec![
                    version("release", 5, "release"),
                    version("beta", 25, "beta")
                ]
            )
            .as_deref(),
            Some("beta"),
            "a removed beta is not traded for an older Release from an earlier line"
        );
        assert_eq!(
            pick(
                ModChannel::Beta,
                vec![
                    version("release", 30, "release"),
                    version("beta", 25, "beta")
                ]
            )
            .as_deref(),
            Some("release"),
            "a Release newer than every remaining beta still wins"
        );
        assert_eq!(
            pick(
                ModChannel::Stable,
                vec![
                    version("old", 5, "release"),
                    version("new", 15, "release"),
                    version("beta", 20, "beta")
                ]
            )
            .as_deref(),
            Some("new"),
            "a removed Release gets the newest Release"
        );
        assert_eq!(
            pick(
                ModChannel::Alpha,
                vec![
                    version("release", 5, "release"),
                    version("beta", 15, "beta")
                ]
            )
            .as_deref(),
            Some("beta"),
            "a removed alpha gets the newest file, not the oldest Release"
        );
        assert_eq!(
            chosen_id(choose_update(
                vec![
                    version("release", 30, "release"),
                    version("beta", 25, "beta")
                ],
                &[gone(ModChannel::Stable)],
                &policy(&[
                    (ModChannel::Stable, false),
                    (ModChannel::Beta, true),
                    (ModChannel::Alpha, false),
                ])
            ))
            .as_deref(),
            Some("beta"),
            "channels that do not allow updates are skipped"
        );
    }

    #[test]
    fn a_file_removed_from_one_platform_follows_its_listing_on_the_other() {
        let installed = [
            Installed::Gone {
                id: InstalledId::Curseforge {
                    project_id: 42,
                    file_id: 200,
                },
                channel: ModChannel::Beta,
            },
            Installed::present(version("installed", 20, "beta")),
        ];
        assert_eq!(
            chosen_id(choose_update(
                vec![cf_file(150, 5, 1), cf_file(250, 25, 2)],
                &installed,
                &sources()
            ))
            .as_deref(),
            Some("250"),
            "CurseForge files uploaded before the removed one do not replace it"
        );
        assert_eq!(
            chosen_id(choose_update(
                vec![cf_file(150, 5, 1), cf_file(250, 25, 2), cf_file(300, 22, 1)],
                &installed,
                &sources()
            ))
            .as_deref(),
            Some("300"),
            "a newer Release is still preferred over a newer beta"
        );
        assert_eq!(
            chosen_id(choose_update(
                vec![cf_file(150, 5, 1)],
                &installed,
                &sources()
            )),
            None
        );
    }

    #[test]
    fn a_removed_curseforge_file_is_followed_by_the_files_uploaded_after_it() {
        let cf_gone = |channel| Installed::Gone {
            id: InstalledId::Curseforge {
                project_id: 42,
                file_id: 100,
            },
            channel,
        };
        // Modrinth listed the same jar on day 30, after CurseForge's file 200
        // (day 20) followed the removed file 100.
        assert_eq!(
            chosen_id(choose_update(
                vec![cf_file(200, 20, 1), version("installed", 30, "release")],
                &[
                    cf_gone(ModChannel::Stable),
                    Installed::present(version("installed", 30, "release")),
                ],
                &sources()
            ))
            .as_deref(),
            Some("200"),
            "the other platform's later listing does not hide a newer CurseForge file"
        );
        // A removed beta still moves to a newer Release first.
        assert_eq!(
            chosen_id(choose_update(
                vec![cf_file(50, 5, 1), cf_file(200, 25, 1), cf_file(300, 28, 2)],
                &[cf_gone(ModChannel::Beta)],
                &sources()
            ))
            .as_deref(),
            Some("200"),
            "the newer Release, never the older one or a newer beta"
        );
        assert_eq!(
            chosen_id(choose_update(
                vec![cf_file(50, 5, 1)],
                &[cf_gone(ModChannel::Beta)],
                &sources()
            )),
            None,
            "a file uploaded before the removed one does not replace it"
        );
    }

    #[test]
    fn a_jar_is_held_to_its_most_stable_platform_label() {
        assert_eq!(
            installed_channel([Some(ModChannel::Stable), Some(ModChannel::Beta)]),
            Some(ModChannel::Stable)
        );
        assert_eq!(
            installed_channel([Some(ModChannel::Alpha), Some(ModChannel::Beta)]),
            Some(ModChannel::Beta)
        );
        assert_eq!(
            installed_channel([None, Some(ModChannel::Beta)]),
            Some(ModChannel::Beta),
            "an unlisted or unreadable label is ignored"
        );
        assert_eq!(installed_channel([None, None]), None);

        // CurseForge calls the installed jar a Release, Modrinth a Beta.
        let channel = installed_channel([Some(ModChannel::Stable), Some(ModChannel::Beta)])
            .expect("both labels are readable");
        let installed = [
            Installed::Present {
                version: cf_file(100, 10, 1),
                channel,
            },
            Installed::Present {
                version: version("installed", 10, "beta"),
                channel,
            },
        ];
        assert_eq!(
            chosen_id(choose_update(
                vec![
                    cf_file(100, 10, 1),
                    cf_file(200, 20, 2),
                    version("installed", 10, "beta"),
                    version("newer-beta", 20, "beta"),
                ],
                &installed,
                &sources()
            )),
            None,
            "neither platform moves the Release to a beta"
        );
        for paths in ["1.20.1,fabric,beta", "1.20.1,fabric,alpha"] {
            assert!(!fabric().has_path("mods", paths, &sources(), channel));
        }
    }

    #[test]
    fn task11_shader_policy_requires_the_detected_engine() {
        let context = UpdateContext {
            game: "1.20.1".into(),
            loaders: vec!["fabric".into()],
            shader_engines: vec!["iris".into()],
        };
        assert!(
            context.has_path(
                "shaders",
                "1.20.1,shaders,stable",
                &sources(),
                ModChannel::Stable
            ),
            "CF typed shader path with detected engine"
        );
        assert!(context.compatible("shaders", &["1.20.1".into()], &["iris".into()]));
        assert!(
            !context.compatible("shaders", &["1.20.1".into()], &["fabric".into()]),
            "a modloader alone is not a shader engine"
        );
        let absent = UpdateContext {
            shader_engines: vec![],
            ..context
        };
        assert!(!absent.has_path(
            "shaders",
            "1.20.1,shaders,stable",
            &sources(),
            ModChannel::Stable
        ));
    }

    #[test]
    fn task11_channel_and_type_normal_controls() {
        let mut policy = sources();
        let installed = [Installed::present(version("installed", 10, "release"))];
        let candidates = || {
            vec![
                version("stable", 20, "release"),
                version("beta", 25, "beta"),
            ]
        };
        assert!(
            matches!(choose_update(candidates(), &installed, &policy), Some(RemoteVersion::Modrinth(v)) if v.id == "stable")
        );
        policy.channels[0].allow_updates = false;
        assert!(
            matches!(choose_update(candidates(), &installed, &policy), Some(RemoteVersion::Modrinth(v)) if v.id == "beta")
        );
        policy
            .platform_blacklist
            .push(carbon_platforms::ModPlatform::Modrinth);
        assert!(choose_update(candidates(), &installed, &policy).is_none());
        let vanilla = UpdateContext {
            game: "1.20.1".into(),
            loaders: vec![],
            shader_engines: vec![],
        };
        for kind in ["resourcepacks", "datapacks"] {
            assert!(vanilla.compatible(kind, &["1.20.1".into()], &["minecraft".into()]));
            assert!(vanilla.cf_compatible(kind, &["1.20.1".into()]));
            assert!(!vanilla.compatible(kind, &["1.20.2".into()], &["minecraft".into()]));
        }
        assert!(!vanilla.cf_compatible("mods", &["1.20.1".into(), "Fabric".into()]));
        assert!(!vanilla.cf_compatible("shaders", &["1.20.1".into()]));
    }
}
