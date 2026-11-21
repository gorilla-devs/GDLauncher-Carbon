import { el, svgDoc } from "./svg.mjs"

/**
 * The mock's `.frost` noise, masked by its radial fade from one corner.
 * `corner` is "bl" (230 px, 0 -> .5 at 90 px -> 0) or "tr" (170 px, 0 -> .4 at
 * 60 px -> 0). Opacity .55 as in the mock.
 */
function frost(size, cx, cy, mid, midAlpha) {
  const mask = el(
    "mask",
    {
      id: "fm",
      maskUnits: "userSpaceOnUse",
      x: 0,
      y: 0,
      width: size,
      height: size
    },
    [
      el(
        "radialGradient",
        { id: "ff", cx, cy, r: size, gradientUnits: "userSpaceOnUse" },
        [
          el("stop", { offset: 0, "stop-color": "#fff", "stop-opacity": 1 }),
          el("stop", {
            offset: mid / size,
            "stop-color": "#fff",
            "stop-opacity": midAlpha
          }),
          el("stop", { offset: 1, "stop-color": "#fff", "stop-opacity": 0 })
        ]
      ),
      el("rect", { width: size, height: size, fill: "url(#ff)" })
    ]
  )
  const filter = el(
    "filter",
    {
      id: "fn",
      filterUnits: "userSpaceOnUse",
      x: 0,
      y: 0,
      width: size,
      height: size
    },
    [
      el("feTurbulence", {
        type: "fractalNoise",
        baseFrequency: 0.8,
        numOctaves: 3,
        stitchTiles: "stitch"
      }),
      el("feColorMatrix", {
        values: "0 0 0 0 1 0 0 0 0 1 0 0 0 0 1 0 0 0 1.6 -0.62"
      })
    ]
  )
  return svgDoc({ width: size, height: size, viewBox: `0 0 ${size} ${size}` }, [
    el("defs", {}, [filter, mask]),
    el(
      "g",
      { mask: "url(#fm)", opacity: 0.55 },
      el("rect", { width: size, height: size, filter: "url(#fn)" })
    )
  ])
}

export const frostBottomLeft = () => frost(230, 0, 230, 90, 0.5)
export const frostTopRight = () => frost(170, 170, 0, 60, 0.4)
