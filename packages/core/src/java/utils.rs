
#[cfg(target_os = "windows")]
pub fn get_path_separator() -> char {
        ';'
}

#[cfg(not(target_os = "windows"))]
pub fn get_path_separator() -> char {
        ':'
}