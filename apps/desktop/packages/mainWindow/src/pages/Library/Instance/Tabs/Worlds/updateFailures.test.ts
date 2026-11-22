import { beforeEach, expect, it, vi } from "vitest"

const error = vi.hoisted(() => vi.fn())
vi.mock("@gd/ui", () => ({ toast: { error } }))
vi.mock("@gd/i18n", () => ({
  i18n: {
    t: (key: string, options?: { places?: string; filename?: string }) =>
      options?.places
        ? `${key} ${options.places}`
        : options?.filename
          ? `${key} ${options.filename}`
          : key
  }
}))
import { toastUpdateFailures } from "./updateFailures"

beforeEach(() => error.mockClear())

const updated = (place: string | null) => ({ place, ok: true, error: null })

it("says nothing when every copy was updated", () => {
  toastUpdateFailures([updated("My World"), updated(null)])
  expect(error).not.toHaveBeenCalled()
})

it("words the first failed place from the code of its refusal", () => {
  toastUpdateFailures([
    updated(null),
    {
      place: "My World",
      ok: false,
      error: "This world already contains that datapack filename",
      error_code: "DATAPACK_NAME_TAKEN",
      error_data: { filename: "Tom & Jerry.zip" }
    },
    {
      place: "Skyblock",
      ok: false,
      error: "Selected world no longer exists",
      error_code: "WORLD_GONE"
    }
  ])
  expect(error.mock.calls).toEqual([
    [
      "content:_trn_worlds.update_failed_in My World, Skyblock",
      { description: "content:_trn_worlds.refusal.name_taken Tom & Jerry.zip" }
    ]
  ])
})

it("keeps the core's sentence for a failed place without a code", () => {
  toastUpdateFailures([
    { place: null, ok: false, error: "The update failed", error_code: null }
  ])
  expect(error.mock.calls).toEqual([
    [
      "content:_trn_worlds.update_failed_in content:_trn_worlds.instance_copy",
      { description: "The update failed" }
    ]
  ])
})

it("shows a failed place given as a folder name by its in-game name", () => {
  toastUpdateFailures(
    [
      { place: "New World", ok: false, error: "x", error_code: null },
      { place: "Creative", ok: false, error: "x", error_code: null },
      { place: "Gone", ok: false, error: "x", error_code: null }
    ],
    [
      { name: "New World", title: "Skyblock" },
      { name: "Creative", title: null }
    ]
  )
  expect(error.mock.calls[0][0]).toBe(
    "content:_trn_worlds.update_failed_in Skyblock, Creative, Gone"
  )
})
