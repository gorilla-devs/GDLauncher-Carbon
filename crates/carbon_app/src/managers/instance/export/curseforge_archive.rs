use std::{path::PathBuf, sync::Arc};

use anyhow::anyhow;

use crate::{
    domain::instance::InstanceId,
    managers::{
        instance::{InstanceType, InvalidInstanceIdError},
        AppInner,
    },
};

pub async fn export_curseforge(
    app: Arc<AppInner>,
    instance_id: InstanceId,
    save_path: PathBuf,
) -> anyhow::Result<()> {
    let instances = app.instance_manager().instances.read().await;
    let instance = instances
        .get(&instance_id)
        .ok_or(InvalidInstanceIdError(instance_id))?;

    let InstanceType::Valid(data) = &mut instance.type_ else {
        return Err(anyhow!("Instance {instance_id} is not in a valid state"));
    };

    let config = data.config.clone();

    Ok(())
}
