/**
 * @type {import('electron-builder').Configuration}
 * @see https://www.electron.build/configuration/configuration
 */
module.exports = {
  productName: "GDLauncher Carbon",
  appId: "org.gorilladevs.GDLauncherCarbon",
  copyright: "Copyright © 2022 ${author}",
  asar: true,
  directories: {
    output: "release/${version}",
    buildResources: "build",
  },
  files: ["dist"],
  npmRebuild: false,
  protocols: [
    {
      name: "gdlauncher",
      role: "Viewer",
      schemes: ["gdlauncher"],
    },
  ],
  win: {
    target: [
      {
        target: "nsis",
      },
    ],
    artifactName: "${productName}-${version}-Setup.${ext}",
  },
  nsis: {
    oneClick: false,
    perMachine: false,
    allowToChangeInstallationDirectory: true,
    deleteAppDataOnUninstall: false,
  },
  mac: {
    target: {
      target: "dmg",
    },
    artifactName: "${productName}-${version}-${arch}-Installer.${ext}",
    entitlements: "./entitlements.mac.plist",
    entitlementsInherit: "./entitlements.mac.plist",
  },
  linux: {
    target: ["AppImage"],
    artifactName: "${productName}-${version}-Installer.${ext}",
  },
  beforePack: async (context) => {
    const { spawnSync } = require("child_process");
    const { promises: fs } = require("fs");

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
        spawnSync("pnpm", ["core-build", "-- linux32-x64"], {
          stdio: "inherit",
          shell: true,
          cwd: "../../",
        });
      }
    }

    await fs.copyFile("../../core/core.node", "dist/preload/core.node");
  },
};
