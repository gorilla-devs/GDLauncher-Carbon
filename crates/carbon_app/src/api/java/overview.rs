//! Frontend types for the Java overview behind Settings › Java.

use super::FEJavaComponentType;
use crate::managers::java::{
    instance_java::JavaUsage,
    overview::{AdvisoryFix, JavaEntry, JavaGroup, JavaNativity, JavaOverview, KnownIssue},
    overview_actions::RemovedUnusedJavas,
};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;

/// `u32` because specta refuses `u64`.
fn saturating_u32(n: u64) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

#[derive(Type, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEJavaOverview {
    pub groups: Vec<FEJavaGroup>,
    pub other_javas: Vec<FEJavaEntry>,
    pub unused_java_ids: Vec<String>,
    pub unused_size_bytes: u32,
    /// Instances naming each custom profile, by name; absent means none.
    pub custom_profile_users: HashMap<String, u32>,
    /// Every entry's `usedBy` is the whole story; false until the startup
    /// instance scan is done.
    pub usage_complete: bool,
}

impl From<JavaOverview> for FEJavaOverview {
    fn from(overview: JavaOverview) -> Self {
        Self {
            groups: overview.groups.into_iter().map(Into::into).collect(),
            other_javas: overview.other_javas.into_iter().map(Into::into).collect(),
            unused_java_ids: overview.unused_java_ids,
            unused_size_bytes: saturating_u32(overview.unused_size_bytes),
            custom_profile_users: overview.custom_profile_users,
            usage_complete: overview.usage_complete,
        }
    }
}

#[derive(Type, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEJavaGroup {
    pub major: u8,
    pub from_version: String,
    pub to_version: String,
    pub newest: bool,
    pub oldest: bool,
    pub in_use_java_id: Option<String>,
    pub user_chosen: bool,
    pub downloads_when_needed: bool,
    pub javas: Vec<FEJavaEntry>,
}

impl From<JavaGroup> for FEJavaGroup {
    fn from(group: JavaGroup) -> Self {
        Self {
            major: group.major,
            from_version: group.from_version,
            to_version: group.to_version,
            newest: group.newest,
            oldest: group.oldest,
            in_use_java_id: group.in_use_java_id,
            user_chosen: group.user_chosen,
            downloads_when_needed: group.downloads_when_needed,
            javas: group.javas.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Type, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEJavaEntry {
    pub id: String,
    pub path: String,
    pub version: String,
    pub major: u8,
    pub vendor_name: String,
    pub source: FEJavaComponentType,
    pub size_bytes: Option<u32>,
    pub is_valid: bool,
    pub nativity: FEJavaNativity,
    pub pinned: bool,
    pub update_to: Option<String>,
    pub known_issues: Vec<FEJavaKnownIssue>,
    pub used_by: Vec<FEJavaUsage>,
    pub running_game: bool,
    pub pending_removal: bool,
}

impl From<JavaEntry> for FEJavaEntry {
    fn from(entry: JavaEntry) -> Self {
        Self {
            id: entry.id,
            path: entry.path,
            version: entry.version,
            major: entry.major,
            vendor_name: entry.vendor_name,
            source: entry.source.into(),
            size_bytes: entry.size_bytes.map(saturating_u32),
            is_valid: entry.is_valid,
            nativity: entry.nativity.into(),
            pinned: entry.pinned,
            update_to: entry.update_to,
            known_issues: entry.known_issues.into_iter().map(Into::into).collect(),
            used_by: entry.used_by.into_iter().map(Into::into).collect(),
            running_game: entry.running_game,
            pending_removal: entry.pending_removal,
        }
    }
}

#[derive(Type, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FEJavaNativity {
    Native,
    Rosetta,
    X86OnX64,
    Other,
}

impl From<JavaNativity> for FEJavaNativity {
    fn from(nativity: JavaNativity) -> Self {
        match nativity {
            JavaNativity::Native => Self::Native,
            JavaNativity::Rosetta => Self::Rosetta,
            JavaNativity::X86OnX64 => Self::X86OnX64,
            JavaNativity::Other => Self::Other,
        }
    }
}

#[derive(Type, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEJavaKnownIssue {
    pub advisory_id: String,
    pub name: String,
    pub url: String,
    pub fixed_version: Option<String>,
    pub fix: Option<FEAdvisoryFix>,
    /// Games GDLauncher starts on this Java are protected from it at launch
    /// until it is updated.
    pub mitigated: bool,
}

impl From<KnownIssue> for FEJavaKnownIssue {
    fn from(issue: KnownIssue) -> Self {
        Self {
            advisory_id: issue.advisory_id,
            name: issue.name,
            url: issue.url,
            fixed_version: issue.fixed_version,
            fix: issue.fix.map(Into::into),
            mitigated: issue.mitigated,
        }
    }
}

#[derive(Type, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FEAdvisoryFix {
    Update {
        version: String,
    },
    SwitchToAzul {
        #[serde(rename = "buildId")]
        build_id: String,
        version: String,
    },
}

impl From<AdvisoryFix> for FEAdvisoryFix {
    fn from(fix: AdvisoryFix) -> Self {
        match fix {
            AdvisoryFix::Update { version } => Self::Update { version },
            AdvisoryFix::SwitchToAzul { build_id, version } => {
                Self::SwitchToAzul { build_id, version }
            }
        }
    }
}

#[derive(Type, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEJavaUsage {
    pub instance_id: u32,
    pub name: String,
    pub direct: bool,
    /// The custom profile the instance uses the Java through.
    pub via_profile: Option<String>,
}

impl From<JavaUsage> for FEJavaUsage {
    fn from(usage: JavaUsage) -> Self {
        Self {
            instance_id: u32::try_from(usage.instance_id.0).unwrap_or_default(),
            name: usage.name,
            direct: usage.direct,
            via_profile: usage.via_profile,
        }
    }
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FESetMajorDefaultArgs {
    pub major: u8,
    /// `null` chooses automatically again.
    pub java_id: Option<String>,
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FESwitchJavaToFixedArgs {
    pub java_id: String,
    pub build_id: String,
}

#[derive(Type, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FERemovedUnusedJavas {
    pub count: u32,
    /// A game is running, so they're removed when it closes.
    pub deferred: bool,
}

impl From<RemovedUnusedJavas> for FERemovedUnusedJavas {
    fn from(removed: RemovedUnusedJavas) -> Self {
        Self {
            count: removed.count,
            deferred: removed.deferred,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixes_and_nativity_serialize_as_the_frontend_expects() {
        assert_eq!(
            serde_json::to_value(FEAdvisoryFix::SwitchToAzul {
                build_id: "b".into(),
                version: "21.0.13.1".into()
            })
            .unwrap(),
            serde_json::json!({ "switchToAzul": { "buildId": "b", "version": "21.0.13.1" } })
        );
        assert_eq!(
            serde_json::to_value(FEAdvisoryFix::Update {
                version: "21.0.13.1".into()
            })
            .unwrap(),
            serde_json::json!({ "update": { "version": "21.0.13.1" } })
        );
        assert_eq!(
            serde_json::to_value(FEJavaNativity::X86OnX64).unwrap(),
            serde_json::json!("x86OnX64")
        );
    }
}
