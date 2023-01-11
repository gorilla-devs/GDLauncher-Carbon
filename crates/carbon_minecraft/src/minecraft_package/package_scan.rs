use cairo::path::Path;
use crate::minecraft_package::MinecraftPackage;

enum MinecraftPackageScanError{


}

pub(crate) type MinecraftPackageScanResult = Result<MinecraftPackage, MinecraftPackageScanError>;

pub(crate) trait MinecraftPackageScanner {
    async fn scan_for_packages(package_dir_path: &impl AsRef<Path>) -> MinecraftPackageScanResult {
        todo!()
    }
}



