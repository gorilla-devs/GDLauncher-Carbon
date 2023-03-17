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
  copyright: `Copyright © ${new Date().getFullYear()} GorillaDevs Inc.`,
  asar: true,
  directories: {
    output: "release",
    buildResources: "build",
  },
  files: ["dist", "package.json"],
  extraResources: [
    {
      from: "../../target/release/core_module.exe",
      to: "core_module.exe",
    },
    {
      from: "../../target/release/core_module",
      to: "core_module",
    },
    {
      from: "./JavaCheck.class",
      to: "JavaCheck.class",
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
    // target: ["dir", "zip", "nsis"],
    target: ["dir"],
    artifactName: "${productName}-${version}-${arch}-Setup.${ext}",
  },
  nsis: {
    oneClick: false,
    perMachine: false,
    allowToChangeInstallationDirectory: true,
    deleteAppDataOnUninstall: false,
  },
  mac: {
    // target: ["dir", "zip", "dmg"],
    target: ["dir"],
    artifactName: "${productName}-${version}-${arch}-Installer.${ext}",
    entitlements: "./entitlements.mac.plist",
    entitlementsInherit: "./entitlements.mac.plist",
  },
  linux: {
    target: isDockerBuild ? ["dir"] : ["dir"],
    artifactName: "${productName}-5.0.0-${arch}-Installer.${ext}",
  },
  afterAllArtifactBuild: () => {
    const path = require("path");
    const fs = require("fs");

    const packageJsonPath = path.join(__dirname, "package.json");
    const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, "utf8"));

    const version = packageJson.version;

    if (!version) {
      throw new Error(
        "App version removed before end of build! May be corrupted!"
      );
    }

    delete packageJson.version;

    fs.writeFileSync(
      packageJsonPath,
      `${JSON.stringify(packageJson, null, 2)}\n`,
      "utf8"
    );
  },
};
