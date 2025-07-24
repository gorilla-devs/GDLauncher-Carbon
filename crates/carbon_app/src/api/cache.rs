use super::Set;
use crate::{
    api::{
        keys::{
            self,
            cache::{
                CLEAR_CACHE_HISTORY, GET_CACHE_HISTORY, GET_CACHE_STATUS, GET_CACHE_STATS,
            },
        },
        router::router,
    },
    managers::App,
};
use chrono::{DateTime, Utc};
use rspc::RouterBuilder;
use serde::{Deserialize, Serialize};
use specta::Type;

pub(super) fn mount() -> RouterBuilder<App> {
    router! {
        query GET_CACHE_STATUS[app, _args: ()] {
            let cache_manager = app.meta_cache_manager();
            let current_tasks = cache_manager.get_current_tasks();
            
            Ok(FECacheStatus {
                current_tasks: current_tasks.into_iter().map(|task| FECacheTask {
                    id: task.id,
                    task_type: FECacheTaskType::from(task.task_type),
                    started_at: task.started_at,
                    status: FECacheTaskStatus::from(task.status),
                }).collect(),
            })
        }

        query GET_CACHE_HISTORY[app, _args: ()] {
            let cache_manager = app.meta_cache_manager();
            let history = cache_manager.get_task_history();
            
            Ok(FECacheHistory {
                tasks: history.into_iter().map(|task| FECacheTaskHistory {
                    id: task.id,
                    task_type: FECacheTaskType::from(task.task_type),
                    started_at: task.started_at,
                    completed_at: task.completed_at,
                    duration_ms: task.duration_ms,
                    success: task.success,
                    error_message: task.error_message,
                    details: task.details,
                }).collect(),
            })
        }

        query GET_CACHE_STATS[app, _args: ()] {
            let cache_manager = app.meta_cache_manager();
            let stats = cache_manager.get_task_stats();
            
            Ok(FECacheStats {
                current_tasks: stats.current_tasks,
                total_completed: stats.total_completed,
                successful: stats.successful,
                failed: stats.failed,
                average_duration_ms: stats.average_duration_ms,
            })
        }

        mutation CLEAR_CACHE_HISTORY[app, _args: ()] {
            let cache_manager = app.meta_cache_manager();
            cache_manager.clear_task_history();
            Ok(())
        }
    }
}

#[derive(Type, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FECacheStatus {
    pub current_tasks: Vec<FECacheTask>,
}

#[derive(Type, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FECacheHistory {
    pub tasks: Vec<FECacheTaskHistory>,
}

#[derive(Type, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FECacheStats {
    pub current_tasks: u32,
    pub total_completed: u32,
    pub successful: u32,
    pub failed: u32,
    pub average_duration_ms: u32,
}

#[derive(Type, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FECacheTask {
    pub id: String,
    pub task_type: FECacheTaskType,
    pub started_at: DateTime<Utc>,
    pub status: FECacheTaskStatus,
}

#[derive(Type, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FECacheTaskHistory {
    pub id: String,
    pub task_type: FECacheTaskType,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub duration_ms: u32,
    pub success: bool,
    pub error_message: Option<String>,
    pub details: Option<String>,
}

#[derive(Type, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum FECacheTaskType {
    FullInstanceScan { instance_name: String, file_count: u32 },
    SingleFileCache { filename: String, instance_name: String },
    ImageExtraction { 
        filename: String, 
        instance_name: String,
        addon_name: Option<String>,
        image_types: Vec<String>,
    },
    PlatformDetection { 
        filename: String, 
        instance_name: String,
        addon_name: Option<String>,
        platform_type: Option<String>,
    },
    UpdateCheck { 
        filename: String, 
        instance_name: String,
        addon_name: Option<String>,
        platform_type: Option<String>,
        current_version: Option<String>,
    },
    CacheClear,
    StartupScan,
}

#[derive(Type, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum FECacheTaskStatus {
    Running { stage: String, progress: Option<FECacheProgress> },
    Completed { success: bool, error_message: Option<String> },
}

#[derive(Type, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FECacheProgress {
    pub current: u32,
    pub total: u32,
}

// Conversion implementations
impl From<crate::managers::metadata::cache::CacheTaskType> for FECacheTaskType {
    fn from(task_type: crate::managers::metadata::cache::CacheTaskType) -> Self {
        use crate::managers::metadata::cache::CacheTaskType;
        match task_type {
            CacheTaskType::FullInstanceScan { instance_name, file_count } => {
                FECacheTaskType::FullInstanceScan { instance_name, file_count }
            }
            CacheTaskType::SingleFileCache { filename, instance_name } => {
                FECacheTaskType::SingleFileCache { filename, instance_name }
            }
            CacheTaskType::ImageExtraction { filename, instance_name, addon_name, image_types } => {
                FECacheTaskType::ImageExtraction { filename, instance_name, addon_name, image_types }
            }
            CacheTaskType::PlatformDetection { filename, instance_name, addon_name, platform_type } => {
                FECacheTaskType::PlatformDetection { filename, instance_name, addon_name, platform_type }
            }
            CacheTaskType::UpdateCheck { filename, instance_name, addon_name, platform_type, current_version } => {
                FECacheTaskType::UpdateCheck { filename, instance_name, addon_name, platform_type, current_version }
            }
            CacheTaskType::CacheClear => FECacheTaskType::CacheClear,
            CacheTaskType::StartupScan => FECacheTaskType::StartupScan,
        }
    }
}

impl From<crate::managers::metadata::cache::CacheTaskStatus> for FECacheTaskStatus {
    fn from(status: crate::managers::metadata::cache::CacheTaskStatus) -> Self {
        use crate::managers::metadata::cache::CacheTaskStatus;
        match status {
            CacheTaskStatus::Running { stage, progress } => {
                FECacheTaskStatus::Running {
                    stage,
                    progress: progress.map(|(current, total)| FECacheProgress { current, total }),
                }
            }
            CacheTaskStatus::Completed { success, error_message } => {
                FECacheTaskStatus::Completed { success, error_message }
            }
        }
    }
}