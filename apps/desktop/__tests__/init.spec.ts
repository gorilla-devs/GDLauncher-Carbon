import { expect, test } from "@playwright/test";
import fs from "fs";
import {
  clickMenuItemById,
  ipcMainCallFirstListener,
  ipcRendererCallFirstListener,
  ipcMainInvokeHandler,
  ipcRendererInvoke,
} from "electron-playwright-helpers";
import { version } from "../package.json";
import path from "path";
import { ElectronApplication, Page, _electron as electron } from "playwright";

let electronApp: ElectronApplication;

const isArm64 = () => {
  let arm64 = true;
  try {
    fs.accessSync(path.join(__dirname, `../release/${version}/mac-arm64`));
  } catch {
    arm64 = false;
  }
  return arm64;
};

const getBinaryPath = async () => {
  let basePath = path.join(__dirname, `../release/${version}/`);

  if (process.platform === "win32") {
    basePath = path.join(basePath, "win-unpacked", "GDLauncher Carbon.exe");
  } else if (process.platform === "linux") {
    basePath = path.join(basePath, "linux-unpacked", "@gddesktop");
  } else if (process.platform === "darwin") {
    basePath = path.join(
      basePath,
      isArm64() ? "mac-arm64" : "mac",
      "GDLauncher Carbon.app",
      "Contents",
      "MacOS",
      "GDLauncher Carbon"
    );
  }

  return basePath;
};

test.describe("Init Tests", () => {
  test.skip(() => isArm64(), "Only x64 is supported");

  test.beforeAll(async () => {
    // set the CI environment variable to true
    process.env.CI = "e2e";
    electronApp = await electron.launch({
      args: [],
      executablePath: await getBinaryPath(),
    });

    page = await electronApp.firstWindow();
    await new Promise((resolve) => setTimeout(resolve, 500));

    const innerText = await (await page.$(".appFatalCrashState"))?.innerHTML();
    expect(innerText).toBe(undefined);
  });

  test.afterAll(async () => {
    await electronApp.close();
  });

  let page: Page;

  test("renders the first page", async () => {
    page = await electronApp.firstWindow();

    // capture errors
    page.on("pageerror", (error) => {
      console.error(error);
    });
    // capture console messages
    page.on("console", (msg) => {
      console.log(msg.text());
    });

    const innerText = await (await page.$(".helloworld"))?.innerHTML();
    expect(innerText).toBe("prova");
    const title = await page.title();
    expect(title).toBe("GDLauncher Carbon");
  });
});
