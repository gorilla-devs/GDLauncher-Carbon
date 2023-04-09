// allow dead code during development to keep warning outputs meaningful
#![allow(dead_code)]

mod minecraft_mod;
pub mod vtask;

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
