const fs = require("fs")
const path = require("path")
const dotenv = require("dotenv")
const { notarize } = require("@electron/notarize")

dotenv.config({
  path: "../../.env",
  quiet: true
})

let arch = process.argv[4].replace(/-/g, "")
let os = process.argv[5].replace(/-/g, "")
let profile = process.argv[7].replace(/-/g, "")

let carbonAppBinName = os === "win" ? "carbon_app.exe" : "carbon_app"
let coreModuleBinName = os === "win" ? "core_module.exe" : "core_module"
let targetTripleLookup = {
  "win-x64": ["x86_64-pc-windows-msvc"],
  "linux-x64": ["x86_64-unknown-linux-gnu"],
  "mac-universal": ["x86_64-apple-darwin", "aarch64-apple-darwin"]
}

let files = targetTripleLookup[`${os}-${arch}`].map((targetTriple) => {
  return {
    from: `../../target/${targetTriple}/${profile}/${carbonAppBinName}`,
    to: `./binaries/${
      targetTriple.includes("aarch") ? "arm64" : "x64"
    }/${coreModuleBinName}`
  }
})

for (const file of files) {
  let dirname = path.dirname(file.to)
  fs.mkdirSync(dirname, { recursive: true })
  fs.copyFileSync(file.from, file.to)
}

let appChannel = require("../../packages/config/version.json").channel

// Select icon based on release channel
let iconName =
  appChannel === "alpha"
    ? "icon_alpha"
    : appChannel === "beta"
      ? "icon_beta"
      : "icon"

let publish =
  appChannel === "snapshot"
    ? undefined
    : {
        provider: "generic",
        url:
          (process.env.GENERIC_PUBLISH_URL || "http://localhost:9000") +
          "/" +
          (process.env.PUBLISH_URL_FOLDER || "")
      }

const appId = "org.gorilladevs.GDLauncher"

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
  // The unmerged mac layout boots through an @electron/universal entry stub that
  // is CommonJS but inherits this package.json, whose "type": "module" would have
  // Node parse the stub's .js as ESM and fail before any app code runs. Every
  // packaged entry is an explicit .cjs, so the declaration only matters to the
  // stub. Scoped to mac to leave the bytes Overwolf signs elsewhere untouched.
  extraMetadata: os === "mac" ? { type: "commonjs" } : undefined,
  extraResources: [
    {
      from: "binaries/${arch}",
      to: `binaries`
    },
    "legal"
  ],
  npmRebuild: false,
  protocols: [
    {
      name: "gdlauncher",
      role: "Viewer",
      schemes: ["gdlauncher"]
    },
    {
      name: "CurseForge",
      role: "Viewer",
      schemes: ["curseforge"]
    },
    {
      name: "Modrinth",
      role: "Viewer",
      schemes: ["modrinth"]
    }
  ],
  win: {
    icon: `build/${iconName}.png`,
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
    icon: `build/${iconName}.png`,
    target: appChannel === "snapshot" ? ["zip"] : ["zip", "dmg"],
    artifactName: "${productName}__${version}__${os}__" + arch + ".${ext}",
    entitlements: "./entitlements.mac.plist",
    extendInfo: "./entitlements.mac.plist",
    minimumSystemVersion: "11.0",
    hardenedRuntime: true,
    gatekeeperAssess: false,
    notarize: false,
    // Overwolf signs the app once per architecture, so the x64 and arm64 asars
    // hold different signatures in package.json. Merging them demands every
    // non-Mach-O file be byte-identical, so ship both asars alongside the
    // arch-dispatching entry stub instead of merging.
    mergeASARs: false
  },
  dmg: {
    sign: false
  },
  linux: {
    icon: `build/${iconName}.png`,
    target:
      appChannel === "snapshot" ? ["zip"] : ["zip", "appImage", "deb", "rpm"],
    artifactName: "${productName}__${version}__${os}__" + arch + ".${ext}",
    category: "Game",
    synopsis: "Custom Minecraft Launcher",
    description:
      "GDLauncher is a custom Minecraft launcher with built-in mod management, modpack support, and a modern interface.",
    maintainer: "GorillaDevs Inc. (support@gdlauncher.com)"
  },
  afterAllArtifactBuild: (_buildResult) => {
    const path = require("path")
    const fs = require("fs")

    const packageJsonPath = path.join(__dirname, "package.json")
    const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, "utf8"))

    packageJson.version = "0.0.0"

    fs.writeFileSync(
      packageJsonPath,
      `${JSON.stringify(packageJson, null, 2)}\n`,
      "utf8"
    )
  },
  afterSign: async (context) => {
    const { electronPlatformName, appOutDir } = context
    if (
      electronPlatformName !== "darwin" ||
      !process.env.APPLE_ID ||
      !process.env.APPLE_APP_SPECIFIC_PASSWORD
    ) {
      console.log("Skipping notarization")
      return
    }

    const appName = context.packager.appInfo.productFilename

    console.log("Notarizing...")
    return await notarize({
      tool: "notarytool",
      appPath: `${appOutDir}/${appName}.app`,
      appleId: process.env.APPLE_ID,
      appleIdPassword: process.env.APPLE_APP_SPECIFIC_PASSWORD,
      teamId: process.env.APPLE_TEAM_ID
    })
  }
}
