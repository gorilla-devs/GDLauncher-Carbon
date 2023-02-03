pub mod app;
pub mod instance;
mod minecraft_mod;
mod minecraft_package;
pub mod modloader;
pub mod account;

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
