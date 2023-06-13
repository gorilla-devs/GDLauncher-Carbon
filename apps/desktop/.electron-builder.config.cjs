let arch = process.argv[4].replace(/-/g, "");
let os = process.argv[5].replace(/-/g, "");
let profile = process.argv[7].replace(/-/g, "");

let carbonAppBinName = os === "win" ? "carbon_app.exe" : "carbon_app";
let coreModuleBinName = os === "win" ? "core_module.exe" : "core_module";
let targetTripleLookup = {
  "win-x64": "x86_64-pc-windows-msvc",
  "linux-x64": "x86_64-unknown-linux-gnu",
  "mac-x64": "x86_64-apple-darwin",
  "mac-arm64": "aarch64-apple-darwin",
};

let targetTriple = targetTripleLookup[`${os}-${arch}`];

let appChannel = require("../../packages/config/version.json").channel;
let publish =
  appChannel === "snapshot"
    ? undefined
    : {
        provider: "generic",
        url: process.env.GENERIC_PUBLISH_URL.replace("${arch}", arch),
      };

module.exports = {
  productName: "GDLauncher",
  appId: "org.gorilladevs.GDLauncher",
  copyright: `Copyright © ${new Date().getFullYear()} GorillaDevs Inc.`,
  publish,
  asar: true,
  directories: {
    output: "release",
    buildResources: "build",
  },
  files: ["dist", "package.json"],
  extraResources: [
    {
      from: `../../target/${targetTriple}/${profile}/${carbonAppBinName}`,
      to: coreModuleBinName,
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
    target: ["dir", "zip", "nsis"],
    artifactName: "${productName}-${version}-" + arch + "-Setup.${ext}",
    verifyUpdateCodeSignature: false,
  },
  nsis: {
    oneClick: false,
    perMachine: false,
    allowToChangeInstallationDirectory: true,
    deleteAppDataOnUninstall: false,
  },
  mac: {
    target: ["dir", "zip", "dmg"],
    artifactName: "${productName}-${version}-" + arch + "-Installer.${ext}",
    entitlements: "./entitlements.mac.plist",
    entitlementsInherit: "./entitlements.mac.plist",
  },
  linux: {
    target: ["dir", "zip"],
    artifactName: "${productName}-${version}-" + arch + "-Installer.${ext}",
  },
  afterAllArtifactBuild: (buildResult) => {
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
