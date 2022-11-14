import { describe, expect, test, assert } from "vitest";
import { fireEvent, render } from "solid-testing-library";
import { Counter } from "../components/Counter";
import { getBasePathUrl } from ".";

describe("Test Utils", () => {
  test("getBasePathUrl", () => {
    assert.equal(
      getBasePathUrl(
        "file:///C:/Users/username/AppData/Local/Programs/obsidian/resources/app.asar/packages/mainWindow/pages/home.tsx"
      ),
      "file:///C:/Users/username/AppData/Local/Programs/obsidian"
    );

    assert.equal(
      getBasePathUrl(
        "file:///Users/someUser/Documents/Obsidian.app/Contents/Resources/app.asar/packages/mainWindow/pages/home.tsx"
      ),
      "file:///Users/someUser/Documents/Obsidian.app/Contents"
    );
  });
});
