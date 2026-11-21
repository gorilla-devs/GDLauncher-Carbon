import { describe, expect, it, vi } from "vitest"
import type { FEThemeBackground } from "@gd/core_module/bindings"
import { backgroundModel, videoAction, warnOnLoadError } from "../background"

const bg = (over: Partial<FEThemeBackground> = {}): FEThemeBackground => ({
  src: "a.png",
  video: false,
  motion: [],
  fit: null,
  position: null,
  dim: null,
  blur: null,
  ...over
})

describe("backgroundModel", () => {
  it("no background, no layer", () => {
    expect(backgroundModel(null, "run")).toBeNull()
  })

  it("applies the defaults", () => {
    const m = backgroundModel(bg(), "run")!
    expect(m.media.kind).toBe("image")
    expect(m.media.src).toBe("a.png")
    expect(m.media.style["object-fit"]).toBe("cover")
    expect(m.media.style["object-position"]).toBe("center")
    expect(m.media.style.filter).toBeUndefined()
    expect(m.media.style.inset).toBe("0")
    expect(m.dim).toBe(0)
  })

  it("uses the theme's fit, position, dim and blur", () => {
    const m = backgroundModel(
      bg({ fit: "contain", position: "bottom right", dim: 0.3, blur: 10 }),
      "run"
    )!
    expect(m.media.style["object-fit"]).toBe("contain")
    expect(m.media.style["object-position"]).toBe("bottom right")
    expect(m.media.style.filter).toBe("blur(10px)")
    expect(m.media.style.inset).toBeUndefined()
    expect(m.media.style.top).toBe("-20px")
    expect(m.media.style.left).toBe("-20px")
    expect(m.media.style.width).toBe("calc(100% + 40px)")
    expect(m.media.style.height).toBe("calc(100% + 40px)")
    expect(m.dim).toBe(0.3)
  })

  it("blur never applies to motion layers", () => {
    const m = backgroundModel(bg({ motion: ["m.svg"], blur: 10 }), "run")!
    expect(m.motion[0].style.filter).toBeUndefined()
    expect(m.motion[0].style.inset).toBe("0")
  })

  it("motion layers only run while motion runs", () => {
    const b = bg({ motion: ["m1.svg", "m2.svg"] })
    expect(backgroundModel(b, "run")!.motion).toHaveLength(2)
    expect(backgroundModel(b, "pause")!.motion).toHaveLength(0)
    expect(backgroundModel(b, "off")!.motion).toHaveLength(0)
  })

  it("image backgrounds never play", () => {
    expect(backgroundModel(bg(), "run")!.playing).toBe(false)
  })

  it("gives the static and motion layers their own compositing layer", () => {
    const m = backgroundModel(bg({ motion: ["m.svg"] }), "run")!
    expect(m.media.style["will-change"]).toBe("transform")
    expect(m.motion[0].style["will-change"]).toBe("transform")
  })

  it("videos play only while motion runs", () => {
    const b = bg({ video: true })
    const run = backgroundModel(b, "run")!
    expect(run.media.kind).toBe("video")
    expect(run.playing).toBe(true)
    expect(backgroundModel(b, "pause")!.playing).toBe(false)
    expect(backgroundModel(b, "off")!.playing).toBe(false)
  })
})

describe("videoAction", () => {
  const run = backgroundModel(bg({ video: true }), "run")!
  const pause = backgroundModel(bg({ video: true }), "pause")!
  const off = backgroundModel(bg({ video: true }), "off")!
  const image = backgroundModel(bg(), "run")!

  it("plays while running", () => {
    expect(videoAction(null, run)).toBe("play")
    expect(videoAction(pause, run)).toBe("play")
  })
  it("pauses on pause and rewinds on off", () => {
    expect(videoAction(run, pause)).toBe("pause")
    expect(videoAction(run, off)).toBe("rewind")
  })
  it("releases when the video goes away", () => {
    expect(videoAction(run, image)).toBe("release")
    expect(videoAction(run, null)).toBe("release")
  })
  it("does nothing for image backgrounds", () => {
    expect(videoAction(image, image)).toBe("none")
    expect(videoAction(null, image)).toBe("none")
    expect(videoAction(null, null)).toBe("none")
  })
})

describe("warnOnLoadError", () => {
  it("names the failed file from the element itself", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {})
    try {
      const onError = warnOnLoadError("image")
      onError({
        currentTarget: {
          src: "file:///opt/gdl/resources/app.asar/dist/assets/scene-1a2b.svg"
        }
      } as unknown as Event)
      expect(warn).toHaveBeenLastCalledWith(
        "Theme background image failed to load: scene-1a2b.svg"
      )
      // an installed theme's file comes from the core, with a token
      onError({
        currentTarget: {
          src: "http://127.0.0.1:4650/theme/asset?id=a&path=assets%2Fsky.png&_token=secret"
        }
      } as unknown as Event)
      expect(warn).toHaveBeenLastCalledWith(
        "Theme background image failed to load: sky.png"
      )
    } finally {
      warn.mockRestore()
    }
  })

  it("copes with an element that has no src", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {})
    try {
      warnOnLoadError("video")({ currentTarget: null } as unknown as Event)
      expect(warn).toHaveBeenCalledWith(
        "Theme background video failed to load: "
      )
    } finally {
      warn.mockRestore()
    }
  })
})
