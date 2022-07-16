/**
 * @type {import('electron-builder').Configuration}
 * @see https://www.electron.build/configuration/configuration
 */
module.exports = {
  productName: "GDLauncher",
  appId: "org.gorilladevs.GDLauncher",
  copyright: "Copyright © 2022 ${author}",
  asar: true,
  directories: {
    output: "release/${version}",
    buildResources: "build",
  },
  files: ["dist"],
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
        arch: ["x64"],
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
      arch: ["x64", "arm64"],
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
        // exec npm run napi-build
        spawnSync("npm", ["run", "napi-build-mac-x86_64"], {
          stdio: "inherit",
          shell: true,
        });
      } else if (context.arch === 3) {
        // arm64
        spawnSync("npm", ["run", "napi-build-mac-aarch64"], {
          stdio: "inherit",
          shell: true,
        });
      }
    } else if (context.electronPlatformName === "win32") {
      if (context.arch === 1) {
        // x64
        // exec npm run napi-build
        spawnSync("npm", ["run", "napi-build-win-x86_64"], {
          stdio: "inherit",
          shell: true,
        });
      }
    }

    await fs.copyFile("packages/napi/napi.node", "dist/preload/napi.node");
  },
};
