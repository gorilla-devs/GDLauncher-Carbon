const fs = require("fs");
const path = require("path");
const dotenv = require("dotenv");

dotenv.config({
  path: "../../.env",
});

let arch = process.argv[4].replace(/-/g, "");
let os = process.argv[5].replace(/-/g, "");
let profile = process.argv[7].replace(/-/g, "");

let carbonAppBinName = os === "win" ? "carbon_app.exe" : "carbon_app";
let coreModuleBinName = os === "win" ? "core_module.exe" : "core_module";
let targetTripleLookup = {
  "win-x64": ["x86_64-pc-windows-msvc"],
  "linux-x64": ["x86_64-unknown-linux-gnu"],
  "mac-universal": ["x86_64-apple-darwin", "aarch64-apple-darwin"],
};

let files = targetTripleLookup[`${os}-${arch}`].map((targetTriple) => {
  return {
    from: `../../target/${targetTriple}/${profile}/${carbonAppBinName}`,
    to: `./binaries/${
      targetTriple.includes("aarch") ? "arm64" : "x64"
    }/${coreModuleBinName}`,
  };
});

for (file of files) {
  let dirname = path.dirname(file.to);
  fs.mkdirSync(dirname, { recursive: true });
  fs.copyFileSync(file.from, file.to);
}

let appChannel = require("../../packages/config/version.json").channel;
let publish =
  appChannel === "snapshot"
    ? undefined
    : {
        provider: "generic",
        url:
          (process.env.GENERIC_PUBLISH_URL || "http://localhost:9000/raw-cdn") +
          "/" +
          process.env.PUBLISH_URL_FOLDER,
      };

module.exports = {
  productName: "GDLauncher",
  appId: "org.gorilladevs.GDLauncher",
  generateUpdatesFilesForAllChannels: true,
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
      from: "binaries/${arch}",
      to: `binaries`,
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
    target: appChannel === "snapshot" ? ["zip"] : ["nsis"],
    artifactName: "${productName}__${version}__${os}__" + arch + ".${ext}",
    verifyUpdateCodeSignature: false,
  },
  nsis: {
    oneClick: false,
    perMachine: false,
    allowToChangeInstallationDirectory: false,
    deleteAppDataOnUninstall: false,
  },
  mac: {
    target: appChannel === "snapshot" ? ["zip"] : ["dmg"],
    artifactName: "${productName}__${version}__${os}__" + arch + ".${ext}",
    entitlements: "./entitlements.mac.plist",
    extendInfo: "./entitlements.mac.bundles.plist",
  },
  linux: {
    target: appChannel === "snapshot" ? ["zip"] : ["appImage"],
    artifactName: "${productName}__${version}__${os}__" + arch + ".${ext}",
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
