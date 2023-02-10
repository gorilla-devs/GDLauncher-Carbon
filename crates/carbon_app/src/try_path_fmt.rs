/// Simple Path formatting with fallback
///
/// # Examples
///
/// ```ignore
/// pub main() {
///     let path = PathBuff::new("./here")
///     assert!(try_path_fmt!("./here"),"./here")
///     let path = PathBuff::new("./here")
/// }
/// ```
macro_rules! try_path_fmt {
    ($path:expr) => {{
        $path
            .as_os_str()
            .to_str()
            .unwrap_or("<<unrepresentable fs path!>>")
    }};
}

pub(crate) use try_path_fmt;
