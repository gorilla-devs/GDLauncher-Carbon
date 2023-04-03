use rspc::RouterBuilderLike;

use crate::api::keys::queue::*;
use crate::managers::queue::TaskHandle;
use crate::managers::App;

use super::router::router;

pub(super) fn mount() -> impl RouterBuilderLike<App> {
    router! {
        query GET_TASKS[app, _: ()] {
            Ok(app.task_queue.get_tasks().await)
        }

        query GET_TASK_STATUS[app, task: TaskHandle] {
            Ok(app.task_queue.get_task_status(task).await)
        }
    }
}
