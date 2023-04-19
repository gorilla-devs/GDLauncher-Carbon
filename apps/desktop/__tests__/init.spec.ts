import { expect, test } from "@playwright/test";
import fs from "fs";
// import {
//   clickMenuItemById,
//   ipcMainCallFirstListener,
//   ipcRendererCallFirstListener,
//   ipcMainInvokeHandler,
//   ipcRendererInvoke,
// } from "electron-playwright-helpers";
import path from "path";
import { ElectronApplication, Page, _electron as electron } from "playwright";
import { getActualUrl } from "./tests_helpers.js";

let electronApp: ElectronApplication;

const isArm64 = () => {
  let arm64 = true;
  try {
    fs.accessSync(`./release/mac-arm64`);
  } catch {
    arm64 = false;
  }
  return arm64;
};

const getRootPath = () => {
  let basePath = `./release/`;

  if (process.platform === "win32") {
    basePath = path.join(basePath, "win-unpacked");
  } else if (process.platform === "linux") {
    basePath = path.join(basePath, "linux-unpacked");
  } else if (process.platform === "darwin") {
    basePath = path.join(
      basePath,
      isArm64() ? "mac-arm64" : "mac",
      "GDLauncher Carbon.app"
    );
  }

  return basePath;
};

const getBinaryPath = async () => {
  let rootPath = getRootPath();

  if (process.platform === "win32") {
    return path.join(rootPath, "GDLauncher Carbon.exe");
  } else if (process.platform === "linux") {
    return path.join(rootPath, "@gddesktop");
  } else if (process.platform === "darwin") {
    return path.join(rootPath, "Contents", "MacOS", "GDLauncher Carbon");
  }
};

const isCoreModulePresent = () => {
  let rootPath = getRootPath();

  if (process.platform === "win32") {
    return fs.existsSync(path.join(rootPath, "resources", "core_module.exe"));
  } else if (process.platform === "linux") {
    return fs.existsSync(path.join(rootPath, "resources", "core_module"));
  } else if (process.platform === "darwin") {
    return fs.existsSync(
      path.join(rootPath, "Contents", "Resources", "core_module")
    );
  }
};

test.describe("Init Tests", () => {
  test.skip(() => isArm64(), "Only x64 is supported on macOS CI");

  test.beforeAll(async () => {
    expect(isCoreModulePresent()).toBeTruthy();

    // set the CI environment variable to true
    process.env.CI = "e2e";
    electronApp = await electron.launch({
      args: [],
      executablePath: await getBinaryPath(),
    });

    page = await electronApp.firstWindow();

    // capture errors
    page.on("pageerror", (error) => {
      console.error(error);
      expect(error).toBeNull();
    });
    // capture console messages
    page.on("console", (msg) => {
      console.log(msg.text());
      // expect(msg.type()).not.toBe("error");
    });

    await new Promise((resolve) => setTimeout(resolve, 3000));

    const errorInnerText = await (
      await page.waitForSelector("#appFatalCrashState")
    )?.innerHTML();
    expect(errorInnerText).toBe(undefined);

    const loadingInnerText = await (
      await page.waitForSelector("#appLoadingState")
    )?.innerHTML();
    expect(loadingInnerText).toBe(undefined);

    const title = await page.title();
    expect(title).toBe("GDLauncher Carbon");
  });

  test.afterAll(async () => {
    await electronApp.close();
  });

  let page: Page;

  test("renders the login page", async () => {
    page = await electronApp.firstWindow();

    const currentUrl = await page.url();
    expect(getActualUrl(currentUrl)).toBe("/");
  });
});
