import { expect, it } from "vitest"
import { MIN_MEMORY_MB, shownMemoryMb, typedMemoryMb } from "./javaMemory"

it("saves a typed memory as whole MB, never under the slider's minimum", () => {
  // 8 typed for 8 GB used to be saved as 8 MB, and no game started.
  expect(typedMemoryMb("8")).toBe(MIN_MEMORY_MB)
  expect(typedMemoryMb("512")).toBe(MIN_MEMORY_MB)
  expect(typedMemoryMb("0")).toBe(MIN_MEMORY_MB)
  expect(typedMemoryMb("1024")).toBe(1024)
  expect(typedMemoryMb(" 6144 ")).toBe(6144)
  expect(typedMemoryMb("99999")).toBe(99999)
})

it("leaves the setting alone for anything that is not a whole number", () => {
  for (const typed of [
    "",
    "  ",
    "abc",
    "4gb",
    "4.5",
    "-2048",
    "1e4",
    "4 096"
  ]) {
    expect(typedMemoryMb(typed), typed).toBeNull()
  }
  // Past what the setting can hold.
  expect(typedMemoryMb("99999999")).toBeNull()
})

it("shows a value saved under the minimum as the minimum games start with", () => {
  expect(shownMemoryMb(8)).toBe(MIN_MEMORY_MB)
  expect(shownMemoryMb(4096)).toBe(4096)
  expect(shownMemoryMb(undefined)).toBeUndefined()
})
