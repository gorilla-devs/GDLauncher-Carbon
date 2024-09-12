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
import { fileURLToPath } from "url";
import { dirname } from "path";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

let electronApp: ElectronApplication;

const getRootPath = () => {
  let basePath = path.resolve(__dirname, "../", "release");

  if (process.platform === "win32") {
    basePath = path.join(basePath, "win-unpacked");
  } else if (process.platform === "linux") {
    basePath = path.join(basePath, "linux-unpacked");
  } else if (process.platform === "darwin") {
    basePath = path.join(basePath, "mac-universal", "GDLauncher.app");
  }

  return basePath;
};

const getBinaryPath = async () => {
  let rootPath = getRootPath();

  if (process.platform === "win32") {
    return path.join(rootPath, "GDLauncher.exe");
  } else if (process.platform === "linux") {
    return path.join(rootPath, "@gddesktop");
  } else if (process.platform === "darwin") {
    return path.join(rootPath, "Contents", "MacOS", "GDLauncher");
  }
};

const isCoreModulePresent = () => {
  let rootPath = getRootPath();

  if (process.platform === "win32") {
    const core_path = path.join(
      rootPath,
      "resources",
      "binaries",
      "core_module.exe"
    );
    console.log("Core module path:", core_path);
    return fs.existsSync(core_path);
  } else if (process.platform === "linux") {
    const core_path = path.join(
      rootPath,
      "resources",
      "binaries",
      "core_module"
    );
    console.log("Core module path:", core_path);
    return fs.existsSync(core_path);
  } else if (process.platform === "darwin") {
    const core_path = path.join(
      rootPath,
      "Contents",
      "Resources",
      "binaries",
      "core_module"
    );
    console.log("Core module path:", core_path);
    return fs.existsSync(core_path);
  }
};

test.describe("Init Tests", () => {
  test.beforeAll(async () => {
    expect(isCoreModulePresent()).toBeTruthy();

    // set the CI environment variable to true
    process.env.CI = "e2e";
    electronApp = await electron.launch({
      args: ["--no-sandbox", "--disable-gpu-sandbox"],
      executablePath: await getBinaryPath(),
      env: { ...process.env, IS_TEST_ENV: true } as any
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

    const rootDiv = await (await page.waitForSelector("#root"))?.innerHTML();
    expect(rootDiv).not.toBeUndefined();

    const errorInnerText = await (
      await page.$("#appFatalCrashState")
    )?.innerHTML();
    expect(errorInnerText).toBeUndefined();

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

    const loginContainer = await (
      await page.waitForSelector("#main-login-page")
    )?.innerHTML();

    expect(loginContainer).not.toBeUndefined();
  });

  // Also test missing core_module
});
