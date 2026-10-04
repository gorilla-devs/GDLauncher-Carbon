import { describe, expect, it } from "vitest"
import {
  changeVersionRefusalText,
  isChangeVersionCancelled,
  offeredVersion
} from "./changeVersionRefusal"

function rspcError(cause: unknown[]) {
  return new Error(
    JSON.stringify({
      code: 500,
      message: JSON.stringify({ cause, backtrace: "" })
    })
  )
}

describe("changeVersionRefusalText", () => {
  it("names the platform that could not be reached", () => {
    const error = rspcError([
      {
        display: "Could not reach Modrinth",
        code: "MODPACK_CHECK_UNREACHABLE",
        data: { platform: "Modrinth" }
      }
    ])
    expect(changeVersionRefusalText(error)).toEqual({
      key: "instances:_trn_change_version_unreachable",
      params: { platform: "Modrinth" }
    })
  })

  it("names no platform when a launcher service failed", () => {
    const error = rspcError([
      { display: "Could not reach", code: "MODPACK_CHECK_UNREACHABLE" }
    ])
    expect(changeVersionRefusalText(error)).toEqual({
      key: "instances:_trn_change_version_unreachable_services"
    })
  })

  it("asks for another version when the requested one is gone", () => {
    const error = rspcError([{ display: "Gone", code: "MODPACK_VERSION_GONE" }])
    expect(changeVersionRefusalText(error)).toEqual({
      key: "instances:_trn_change_version_gone"
    })
  })

  it("leaves any other error to the core's wording", () => {
    expect(
      changeVersionRefusalText(rspcError([{ display: "Something else" }]))
    ).toBeUndefined()
    expect(changeVersionRefusalText(new Error("plain"))).toBeUndefined()
  })
})

describe("isChangeVersionCancelled", () => {
  it("recognises only the cancelled check", () => {
    expect(
      isChangeVersionCancelled(
        rspcError([{ display: "x", code: "MODPACK_CHECK_CANCELLED" }])
      )
    ).toBe(true)
    expect(
      isChangeVersionCancelled(
        rspcError([{ display: "x", code: "MODPACK_VERSION_GONE" }])
      )
    ).toBe(false)
    expect(isChangeVersionCancelled(new Error("plain"))).toBe(false)
  })
})

describe("offeredVersion", () => {
  const versions = [{ id: "a" }, { id: "b" }]

  it("keeps a selection the list offers", () => {
    expect(offeredVersion("b", versions)).toBe("b")
  })

  it("drops a selection the list no longer offers", () => {
    expect(offeredVersion("removed", versions)).toBeNull()
    expect(offeredVersion("a", [])).toBeNull()
    expect(offeredVersion(null, versions)).toBeNull()
  })
})
