import { expect, test } from "@playwright/test";
import {
  clickMenuItemById,
  ipcMainCallFirstListener,
  ipcRendererCallFirstListener,
  ipcMainInvokeHandler,
  ipcRendererInvoke,
} from "electron-playwright-helpers";
import path from "path";
import { ElectronApplication, Page, _electron as electron } from "playwright";

let electronApp: ElectronApplication;

test.beforeAll(async () => {
  // set the CI environment variable to true
  process.env.CI = "e2e";
  electronApp = await electron.launch({
    args: [],
    executablePath: path.join(
      __dirname,
      `../release/1.5.5/mac${
        process.arch === "arm64" ? "-arm64" : ""
      }/GDLauncher Carbon.app/Contents/MacOS/GDLauncher Carbon`
    ),
  });
  electronApp.on("window", async (page) => {
    const filename = page.url()?.split("/").pop();
    console.log(`Window opened: ${filename}`);

    // capture errors
    page.on("pageerror", (error) => {
      console.error(error);
    });
    // capture console messages
    page.on("console", (msg) => {
      console.log(msg.text());
    });
  });
});

test.afterAll(async () => {
  await electronApp.close();
});

let page: Page;

test("renders the first page", async () => {
  page = await electronApp.firstWindow();
  await new Promise((resolve) => setTimeout(resolve, 1000));
  await page.screenshot({ path: 'screenshot.png' });
  const innerText = await (await page.$(".helloworld"))?.innerHTML();
  expect(innerText).toBe("prova");
  const title = await page.title();
  expect(title).toBe("GDLauncher Carbon");
});
