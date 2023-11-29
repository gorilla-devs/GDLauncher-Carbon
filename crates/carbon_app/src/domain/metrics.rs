use serde::{Deserialize, Serialize};

fn skip_serializing_if_zero(value: &u32) -> bool {
    *value == 0
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "event_name", content = "data")]
#[serde(rename_all = "snake_case")]
pub enum Event {
    LauncherStarted,
    PageView {
        page_url: String,
    },
    InstanceInstalled {
        #[serde(skip_serializing_if = "skip_serializing_if_zero")]
        mods_count: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        modloader_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        modloader_version: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        modplatform: Option<String>,
        version: String,
        seconds_taken: u32,
    },
    InstanceLaunched {
        #[serde(skip_serializing_if = "skip_serializing_if_zero")]
        mods_count: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        modloader_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        modloader_version: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        modplatform: Option<String>,
        version: String,
        xmx_memory: u32,
        xms_memory: u32,
        time_to_start_secs: u64,
        timestamp_start: i64,
        timestamp_end: i64,
        timezone_offset: i32,
    },
}
