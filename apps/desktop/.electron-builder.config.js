/**
 * @type {import('electron-builder').Configuration}
 * @see https://www.electron.build/configuration/configuration
 */

const isDockerBuild = process.env.IS_DOCKER_TEST === "true";
console.log(
  "Only generating dir executable for test docker build",
  isDockerBuild
);

module.exports = {
  productName: "GDLauncher Carbon",
  appId: "org.gorilladevs.GDLauncherCarbon",
  copyright: "Copyright © 2022 ${author}",
  asar: true,
  directories: {
    output: "release/${version}",
    buildResources: "build",
  },
  files: ["dist", "package.json"],
  extraResources: [
    {
      from: "../../packages/carbon_core/core.node",
      to: "core.node",
    },
    {
      from: "./JavaCheck.class",
      to: "JavaCheck.class",
    },
    {
      from: "../../target/aarch64-apple-darwin/release/app_helper",
      to: "app_helper",
    },
    {
      from: "../../target/x86_64-apple-darwin/release/app_helper",
      to: "app_helper",
    },
    {
      from: "../../target/x86_64-pc-windows-msvc/release/app_helper.exe",
      to: "app_helper.exe",
    },
    {
      from: "../../target/x86_64-unknown-linux-gnu/release/app_helper",
      to: "app_helper",
    },
  ],
  npmRebuild: false,
  protocols: [
    {
      name: "gdlauncher",
      role: "Viewer",
      schemes: ["gdlauncher"],
    },
  ],
  win: {
    target: ["dir", "zip"],
    artifactName: "${productName}-${version}-${arch}-Setup.${ext}",
  },
  nsis: {
    oneClick: false,
    perMachine: false,
    allowToChangeInstallationDirectory: true,
    deleteAppDataOnUninstall: false,
  },
  mac: {
    target: ["dir", "zip"],
    artifactName: "${productName}-${version}-${arch}-Installer.${ext}",
    entitlements: "./entitlements.mac.plist",
    entitlementsInherit: "./entitlements.mac.plist",
  },
  linux: {
    target: isDockerBuild ? ["dir"] : ["dir", "zip"],
    artifactName: "${productName}-${version}-${arch}-Installer.${ext}",
  },
  beforePack: async (context) => {
    const { spawnSync } = require("child_process");

    if (context.electronPlatformName === "darwin") {
      if (context.arch === 1) {
        // x64
        spawnSync("pnpm", ["core-build", "-- darwin-x64"], {
          stdio: "inherit",
          shell: true,
          cwd: "../../",
        });
      } else if (context.arch === 3) {
        // arm64
        spawnSync("pnpm", ["core-build", "-- darwin-arm64"], {
          stdio: "inherit",
          shell: true,
          cwd: "../../",
        });
      }
    } else if (context.electronPlatformName === "win32") {
      if (context.arch === 1) {
        // x64
        spawnSync("pnpm", ["core-build", "-- win32-x64"], {
          stdio: "inherit",
          shell: true,
          cwd: "../../",
        });
      }
    } else if (context.electronPlatformName === "linux") {
      if (context.arch === 1) {
        // x64
        spawnSync("pnpm", ["core-build", "-- linux-x64"], {
          stdio: "inherit",
          shell: true,
          cwd: "../../",
        });
      }
    }
  },
};
