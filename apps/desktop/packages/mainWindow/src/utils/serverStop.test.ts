import { describe, expect, it } from "vitest"
import {
  FORCE_STOP_AFTER_SECONDS,
  offersForceStop,
  stopClock
} from "./serverStop"

describe("how a server's stop is shown", () => {
  it("offers Force stop only once the stop has taken a while", () => {
    expect(offersForceStop(undefined)).toBe(false)
    expect(offersForceStop(null)).toBe(false)
    expect(offersForceStop(0)).toBe(false)
    expect(offersForceStop(FORCE_STOP_AFTER_SECONDS - 1)).toBe(false)
    expect(offersForceStop(FORCE_STOP_AFTER_SECONDS)).toBe(true)
    expect(offersForceStop(3600)).toBe(true)
  })
  it("shows the time as a clock", () => {
    expect(stopClock(0)).toBe("0:00")
    expect(stopClock(4)).toBe("0:04")
    expect(stopClock(53.9)).toBe("0:53")
    expect(stopClock(67)).toBe("1:07")
    expect(stopClock(3723)).toBe("1:02:03")
    expect(stopClock(-5)).toBe("0:00")
  })
})
