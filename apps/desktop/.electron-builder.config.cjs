const fs = require("fs");
const path = require("path");
const dotenv = require("dotenv");
const { notarize } = require("electron-notarize");

dotenv.config({
  path: "../../.env"
});

let arch = process.argv[4].replace(/-/g, "");
let os = process.argv[5].replace(/-/g, "");
let profile = process.argv[7].replace(/-/g, "");

let carbonAppBinName = os === "win" ? "carbon_app.exe" : "carbon_app";
let coreModuleBinName = os === "win" ? "core_module.exe" : "core_module";
let targetTripleLookup = {
  "win-x64": ["x86_64-pc-windows-msvc"],
  "linux-x64": ["x86_64-unknown-linux-gnu"],
  "mac-universal": ["x86_64-apple-darwin", "aarch64-apple-darwin"]
};

let files = targetTripleLookup[`${os}-${arch}`].map((targetTriple) => {
  return {
    from: `../../target/${targetTriple}/${profile}/${carbonAppBinName}`,
    to: `./binaries/${
      targetTriple.includes("aarch") ? "arm64" : "x64"
    }/${coreModuleBinName}`
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
          process.env.PUBLISH_URL_FOLDER
      };

const appId = "org.gorilladevs.GDLauncher";

module.exports = {
  productName: "GDLauncher",
  appId,
  generateUpdatesFilesForAllChannels: true,
  copyright: `Copyright © ${new Date().getFullYear()} GorillaDevs Inc.`,
  publish,
  asar: true,
  directories: {
    output: "release",
    buildResources: "build"
  },
  files: ["dist", "package.json"],
  extraResources: [
    {
      from: "binaries/${arch}",
      to: `binaries`
    }
  ],
  npmRebuild: false,
  protocols: [
    {
      name: "gdlauncher",
      role: "Viewer",
      schemes: ["gdlauncher"]
    }
  ],
  win: {
    target: appChannel === "snapshot" ? ["zip"] : ["zip", "nsis"],
    artifactName: "${productName}__${version}__${os}__" + arch + ".${ext}",
    verifyUpdateCodeSignature: false
  },
  nsis: {
    oneClick: false,
    perMachine: false,
    allowToChangeInstallationDirectory: false,
    deleteAppDataOnUninstall: false
  },
  mac: {
    target: appChannel === "snapshot" ? ["zip"] : ["zip", "dmg"],
    artifactName: "${productName}__${version}__${os}__" + arch + ".${ext}",
    entitlements: "./entitlements.mac.plist",
    extendInfo: "./entitlements.mac.plist",
    hardenedRuntime: true,
    gatekeeperAssess: false
  },
  dmg: {
    sign: false
  },
  linux: {
    target: appChannel === "snapshot" ? ["zip"] : ["zip", "appImage"],
    artifactName: "${productName}__${version}__${os}__" + arch + ".${ext}"
  },
  afterAllArtifactBuild: (buildResult) => {
    const path = require("path");
    const fs = require("fs");

    const packageJsonPath = path.join(__dirname, "package.json");
    const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, "utf8"));

    packageJson.version = "0.0.0";

    fs.writeFileSync(
      packageJsonPath,
      `${JSON.stringify(packageJson, null, 2)}\n`,
      "utf8"
    );
  },
  afterSign: async (context) => {
    const { electronPlatformName, appOutDir } = context;
    if (electronPlatformName !== "darwin" || !process.env.APPLE_ID || !process.env.APPLE_ID_PASSWORD) {
      console.log("Skipping notarization");
      return;
    }

    const appName = context.packager.appInfo.productFilename;

    console.log("Notarizing...")
    return await notarize({
      appBundleId: appId,
      appPath: `${appOutDir}/${appName}.app`,
      appleId: process.env.APPLE_ID,
      appleIdPassword: process.env.APPLE_ID_PASSWORD
    });
  }
};
