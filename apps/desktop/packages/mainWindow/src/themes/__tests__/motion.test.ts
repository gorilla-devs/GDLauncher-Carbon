import { describe, it, expect } from "vitest"
import { isGameRunning, motionState } from "../motion"

const calm = {
  reducedMotion: false,
  osReducedMotion: false,
  visible: true,
  gameRunning: false
}

describe("motion state", () => {
  it.each([
    [{}, "run"],
    [{ reducedMotion: true }, "off"],
    [{ osReducedMotion: true }, "off"],
    [{ visible: false }, "pause"],
    [{ gameRunning: true }, "pause"],
    [{ reducedMotion: true, visible: false }, "off"],
    [{ osReducedMotion: true, gameRunning: true }, "off"]
  ])("follows its inputs %j", (over, expected) => {
    expect(motionState({ ...calm, ...over })).toBe(expected)
  })
})

describe("game detection", () => {
  const inst = (status: unknown) => ({ status }) as any
  it("detects a running game", () => {
    expect(
      isGameRunning([
        inst({
          status: "valid",
          value: { state: { state: "running", value: {} } }
        })
      ])
    ).toBe(true)
  })
  it("ignores inactive, invalid and missing", () => {
    expect(
      isGameRunning([
        inst({
          status: "valid",
          value: { state: { state: "inactive", value: {} } }
        })
      ])
    ).toBe(false)
    expect(isGameRunning([inst({ status: "invalid", value: {} })])).toBe(false)
    expect(isGameRunning(undefined)).toBe(false)
  })
})
