//! Process-scoped ordered snapshots of the actual current map. Call while the
//! instance map is held; no settings, provider or filesystem await occurs here.
use super::run::LaunchState;
use super::{Instance, InstanceId, InvalidInstanceIdError};
use crate::domain::vtask::VisualTaskId;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Default)]
pub(super) struct RunningPublication(Mutex<u64>);
impl RunningPublication {
    pub(super) fn publish(
        &self,
        instances: &HashMap<InstanceId, Instance>,
        sink: &mut impl FnMut(String),
    ) {
        let mut revision = self.0.lock().expect("running publication ownership");
        *revision += 1;
        let mut ids: Vec<_> = instances
            .iter()
            .filter_map(|(id, instance)| {
                instance
                    .data()
                    .ok()
                    .filter(|data| matches!(data.state, LaunchState::Running(_)))
                    .map(|_| **id)
            })
            .collect();
        ids.sort_unstable();
        sink(format!(
            "_INSTANCE_RUNNING_:{}",
            serde_json::json!({ "revision": *revision, "instances": ids })
        ));
    }
    pub(super) fn change(
        &self,
        instances: &mut HashMap<InstanceId, Instance>,
        id: InstanceId,
        expected: VisualTaskId,
        state: LaunchState,
        action: &str,
        sink: &mut impl FnMut(String),
    ) -> anyhow::Result<()> {
        let instance = instances.get_mut(&id).ok_or(InvalidInstanceIdError(id))?;
        let old = &instance.data()?.state;
        let owned = match old {
            LaunchState::Queued(task) | LaunchState::Preparing(task) => *task == expected,
            LaunchState::Running(running) => running.task_id == Some(expected),
            _ => false,
        };
        if !owned {
            anyhow::bail!("stale worker cannot change current instance state");
        }
        let event = match (&old, &state) {
            (LaunchState::Running(_), LaunchState::Inactive { .. }) => Some("GAME_CLOSED"),
            (LaunchState::Queued(_) | LaunchState::Preparing(_), LaunchState::Running(_)) => {
                Some("GAME_LAUNCHED")
            }
            _ => None,
        };
        instance.data_mut()?.state = state;
        self.publish(instances, sink);
        if let Some(event) = event {
            sink(format!(
                "_INSTANCE_STATE_:{event}|{}|{}|{action}",
                *id, expected.0
            ));
        }
        Ok(())
    }
}
