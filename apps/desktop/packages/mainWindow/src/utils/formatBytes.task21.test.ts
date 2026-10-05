import { expect, it } from "vitest"
import {
  bytePercent,
  formatByteUnit,
  formatBytes,
  parseByteCount
} from "./formatBytes"
it("decimal JSON task bytes retain exact BigInt including beyond JS safe range", () => {
  for (const [raw, expected] of [
    ["5368709120", 5368709120n],
    ["9007199254740991", 9007199254740991n],
    ["9007199254740993", 9007199254740993n],
    ["18446744073709551615", 18446744073709551615n]
  ] as const) {
    expect(
      parseByteCount(JSON.parse(`{"downloaded":"${raw}"}`).downloaded)
    ).toBe(expected)
    expect(bytePercent(raw, raw)).toBe(100)
  }
  expect(() => parseByteCount(9007199254740992)).toThrow("safe")
})
it("byte consumers produce bounded ratios and intentionally rounded units", () => {
  expect(bytePercent("1073741824", "5368709120")).toBe(20)
  expect(bytePercent("4294967296", "5368709120")).toBe(80)
  expect(bytePercent("18446744073709551615", "0")).toBe(0)
  expect(bytePercent("18446744073709551615", "1")).toBe(100)
  expect(formatByteUnit("5368709120", 1048576n, 1)).toBe("5120.0")
  expect(formatBytes("18446744073709551615")).toBe("17179869184.00 GB")
  expect(formatBytes("512")).toBe("512 B")
})
it("existing numeric cache size formatting stays compatible", () => {
  expect(formatBytes(0)).toBe("0 B")
  expect(formatBytes(1024)).toBe("1.0 KB")
  expect(formatBytes(1048576)).toBe("1.0 MB")
  expect(formatBytes(1073741824)).toBe("1.00 GB")
})
