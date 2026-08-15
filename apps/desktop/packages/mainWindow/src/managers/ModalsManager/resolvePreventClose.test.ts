import { describe, it, expect } from "vitest"
import { resolvePreventClose } from "./preventClose"

describe("resolvePreventClose", () => {
  it("blocks when only the instance's live accessor says so (JavaSetup case)", () => {
    // JavaSetup has no registry entry (undefined) but passes preventClose as
    // a ModalLayout prop while on its "automatic" step — the bug this fixes
    // was Escape only ever consulting the registry, which was undefined here.
    expect(
      resolvePreventClose(undefined, { preventCloseAccessor: () => true })
    ).toBe(true)
  })

  it("does not block when the live accessor reports false and there's no registry entry", () => {
    expect(
      resolvePreventClose(undefined, { preventCloseAccessor: () => false })
    ).toBe(false)
  })

  it("blocks when only the static registry says so (AccountBanned case)", () => {
    // AccountBanned never registers an accessor at all (its ModalLayout is
    // never passed a preventClose prop) — the registry alone must still work.
    expect(resolvePreventClose(true, {})).toBe(true)
  })

  it("blocks when the registry is a function that currently returns true", () => {
    expect(resolvePreventClose(() => true, {})).toBe(true)
  })

  it("does not block when neither source says so", () => {
    expect(
      resolvePreventClose(() => false, { preventCloseAccessor: () => false })
    ).toBe(false)
    expect(resolvePreventClose(undefined, {})).toBe(false)
  })

  it("blocks when both sources are present and either is true", () => {
    expect(
      resolvePreventClose(() => false, { preventCloseAccessor: () => true })
    ).toBe(true)
    expect(
      resolvePreventClose(() => true, { preventCloseAccessor: () => false })
    ).toBe(true)
  })

  it("stops blocking once the accessor is unregistered (unmount cleanup)", () => {
    const entry: { preventCloseAccessor?: () => boolean } = {
      preventCloseAccessor: () => true
    }
    expect(resolvePreventClose(undefined, entry)).toBe(true)

    // Simulates ModalLayout's onCleanup clearing its registration.
    entry.preventCloseAccessor = undefined
    expect(resolvePreventClose(undefined, entry)).toBe(false)
  })
})
