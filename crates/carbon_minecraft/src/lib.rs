mod minecraft;

pub mod app;
pub mod instance;
mod minecraft_mod;
mod minecraft_package;
pub mod modloader;

#[macro_export]
macro_rules! try_path_fmt {
    ($path:expr) => {{
        $path
            .as_os_str()
            .to_str()
            .unwrap_or("<<unrepresentable fs path!>>")
    }};
}

/*
#[macro_export]
macro_rules! open_file_async {
    ($path:expr) => {
        {

        }
    };
}

#[macro_export]
macro_rules! read_file_async {
    ($path:expr) => {
        {

        }
    };
}
*/

/*
//
// pub struct Instances {
//     inner: Arc<RwLock<HashMap<String, instance::Instance>>>,
// }
//
// impl Instances {
//     pub async fn scan_for_instances(&mut self, base_path: PathBuf) -> Result<&Instances> {
//         let instances_path = base_path.join("instances");
//
//         if !instances_path.exists() {
//             tokio::fs::create_dir(&instances_path).await.unwrap();
//         }
//
//         let mut instances = self.inner.write().await;
//
//         let mut dirs = tokio::fs::read_dir(instances_path).await?;
//
//         while let Some(entry) = dirs.next_entry().await? {
//             let path = entry.path();
//             if !path.is_dir() {
//                 continue;
//             }
//
//             let Some(name) = path.file_name().map(|name| name.to_string_lossy().to_string()) else {
//                 trace!("Found instance with invalid name: {:?}", path);
//                 continue;
//             };
//
//             let instance = instance::Instance::new(name.clone());
//             instances.insert(name, instance); // impl convenience method for this
//         }
//
//         Ok(&INSTANCES)
//     }
// }
//
// lazy_static! {
//     static ref INSTANCES: Instances = Instances {
//         inner: Arc::new(RwLock::new(HashMap::new()))
//     };
// }
*/
