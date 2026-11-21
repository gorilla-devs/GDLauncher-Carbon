import { describe, it, expect } from "vitest"
import { resultImageUrl } from "./resultImage"

describe("a search result's image", () => {
  const both = { imageUrl: "small.png", highResImageUrl: "large.png" }

  it("the list's small icon prefers the small image, the grid the large", () => {
    expect(resultImageUrl(both, "small")).toBe("small.png")
    expect(resultImageUrl(both, "large")).toBe("large.png")
  })

  it("falls back to the other size when one is missing or empty", () => {
    expect(
      resultImageUrl({ imageUrl: "", highResImageUrl: "large.png" }, "small")
    ).toBe("large.png")
    expect(
      resultImageUrl({ imageUrl: null, highResImageUrl: "large.png" }, "small")
    ).toBe("large.png")
    expect(
      resultImageUrl({ imageUrl: "small.png", highResImageUrl: null }, "large")
    ).toBe("small.png")
  })

  it("is null when there is no image, never an empty src", () => {
    expect(
      resultImageUrl({ imageUrl: null, highResImageUrl: null }, "small")
    ).toBeNull()
    expect(
      resultImageUrl({ imageUrl: "", highResImageUrl: "" }, "large")
    ).toBeNull()
  })
})
