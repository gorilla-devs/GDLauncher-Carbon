import { expect, it } from "vitest"
import { remaining, splitPath } from "./files"

it("splits a kept file into its name and the folder it is in", () => {
  expect(splitPath("config/jei/jei-client.ini")).toEqual({
    name: "jei-client.ini",
    folder: "config/jei"
  })
  expect(splitPath("options.cfg")).toEqual({ name: "options.cfg", folder: "" })
})

it("keeps the files that were not replaced or not asked for", () => {
  expect(
    remaining(
      ["a", "b", "c"],
      [
        { path: "a", error: null },
        { path: "b", error: "held open" }
      ]
    )
  ).toEqual(["b", "c"])
})
