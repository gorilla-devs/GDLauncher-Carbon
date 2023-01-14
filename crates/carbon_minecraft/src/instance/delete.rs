use async_trait::async_trait;
use thiserror::Error;
use crate::instance::{Instance, InstanceStatus};

#[derive(Error, Debug)]
pub enum InstanceDeleteError {
    #[error("unable to delete a non persisted instance!")]
    InstanceNotPersisted,

    #[error("unable to put instance into trash bin : {0} ")]
    InstanceTrashBinPuttingError(#[from] trash::Error),

    #[error("io-error rise while deleting instance : {0}")]
    IoError(#[from] std::io::Error),
}

type InstanceDeleterResult = Result<Instance, InstanceDeleteError>;

#[async_trait]
pub(crate) trait InstanceDeleter {
    async fn delete(instance: Instance, put_in_trash_bin: bool) -> InstanceDeleterResult {
        match &instance.persistence_status {
            InstanceStatus::Persisted(path) if put_in_trash_bin=> {
                trash::delete_all(path)?;
                Ok( Instance::from(instance) )
            },
            InstanceStatus::Persisted(path) if !put_in_trash_bin=> {
                tokio::fs::remove_dir_all(path).await?;
                Ok( Instance::from(instance) )
            },
            _ => Err(InstanceDeleteError::InstanceNotPersisted),
        }
    }
}


