import { el, f, svgDoc } from "./svg.mjs"

/**
 * Santa hat over the real logo, in the logo's own units (the hexagon spans
 * x 0..56, y 0..64). Port of the mock's drawHat with fixed ids. The mock's
 * CSS drop-shadow(0 1.5px 1px) is baked in as feDropShadow, scaled by the
 * 36 px logo's 0.5625 factor back into these units.
 */
export function hat() {
  const defs = [
    el("linearGradient", { id: "hb", x1: 0, y1: 0, x2: 1, y2: 1 }, [
      el("stop", { offset: 0, "stop-color": "#ef4a50" }),
      el("stop", { offset: 0.55, "stop-color": "#d02c38" }),
      el("stop", { offset: 1, "stop-color": "#a3192a" })
    ]),
    el("linearGradient", { id: "hf", x1: 0, y1: 0, x2: 0, y2: 1 }, [
      el("stop", { offset: 0, "stop-color": "#ffffff" }),
      el("stop", { offset: 1, "stop-color": "#d9e2f0" })
    ]),
    el("radialGradient", { id: "hp", cx: 0.38, cy: 0.35, r: 0.75 }, [
      el("stop", { offset: 0, "stop-color": "#ffffff" }),
      el("stop", { offset: 1, "stop-color": "#cdd8ea" })
    ]),
    el(
      "filter",
      {
        id: "hs",
        filterUnits: "userSpaceOnUse",
        x: -8,
        y: -21,
        width: 78,
        height: 60
      },
      el("feDropShadow", {
        dx: 0,
        dy: 2.7,
        stdDeviation: 0.9,
        "flood-color": "#000000",
        "flood-opacity": 0.35
      })
    )
  ]
  const g = []
  g.push(
    el("path", {
      d: "M3 24 C4 7 17 -10 35 -12.5 C47 -14 56.5 -7 60 8.5 C56 1.5 51 -0.5 46.5 0.2 C50 7 52 14 53.5 23 Q28 7 3 24Z",
      fill: "url(#hb)"
    })
  )
  g.push(
    el("path", {
      d: "M60 8.5 C56 1.5 51 -0.5 46.5 0.2 C49.2 4.6 51 9.4 52.4 14.6 C54 10.4 56.8 8.4 60 8.5Z",
      fill: "#86101f"
    })
  )
  g.push(
    el("path", {
      d: "M8.5 18 C9.5 6 19.5 -5.5 33.5 -8.5",
      stroke: "#ff8a8a",
      "stroke-width": 2.4,
      fill: "none",
      opacity: 0.5,
      "stroke-linecap": "round"
    })
  )
  g.push(
    el("path", {
      d: "M31 -7 C39 -7.5 44.5 -4 47 0.5",
      stroke: "#9b1625",
      "stroke-width": 1.3,
      fill: "none",
      opacity: 0.7,
      "stroke-linecap": "round"
    })
  )
  const q = (t, a, b, c) =>
    (1 - t) * (1 - t) * a + 2 * (1 - t) * t * b + t * t * c
  const P0 = [-1.5, 27]
  const C = [28, 6]
  const P2 = [57.5, 27]
  g.push(
    el("path", {
      d:
        "M" +
        P0[0] +
        " " +
        (P0[1] + 1.6) +
        " Q" +
        C[0] +
        " " +
        (C[1] + 1.6) +
        " " +
        P2[0] +
        " " +
        (P2[1] + 1.6),
      stroke: "#a9b8d0",
      "stroke-width": 9.5,
      fill: "none",
      "stroke-linecap": "round"
    })
  )
  g.push(
    el("path", {
      d:
        "M" +
        P0[0] +
        " " +
        P0[1] +
        " Q" +
        C[0] +
        " " +
        C[1] +
        " " +
        P2[0] +
        " " +
        P2[1],
      stroke: "url(#hf)",
      "stroke-width": 9,
      fill: "none",
      "stroke-linecap": "round"
    })
  )
  for (let i = 0; i <= 16; i++) {
    const t = i / 16
    const x = q(t, P0[0], C[0], P2[0])
    const y = q(t, P0[1], C[1], P2[1])
    const dx = 2 * (1 - t) * (C[0] - P0[0]) + 2 * t * (P2[0] - C[0])
    const dy = 2 * (1 - t) * (C[1] - P0[1]) + 2 * t * (P2[1] - C[1])
    const len = Math.hypot(dx, dy)
    let nx = dy / len
    let ny = -dx / len
    if (ny > 0) {
      nx = -nx
      ny = -ny
    }
    const rr = 2.3 + ((i * 7) % 5) * 0.25
    g.push(
      el("circle", {
        cx: f(x + nx * 3.7),
        cy: f(y + ny * 3.7),
        r: rr,
        fill: "#ffffff"
      })
    )
    g.push(
      el("circle", {
        cx: f(x - nx * 3.5),
        cy: f(y - ny * 3.5),
        r: f(rr * 0.85),
        fill: "#e3eaf5"
      })
    )
  }
  const px = 60.2
  const py = 13.6
  g.push(el("circle", { cx: px + 0.6, cy: py + 1.2, r: 6.6, fill: "#9fb0cb" }))
  for (let k = 0; k < 9; k++) {
    const a = (k / 9) * 6.283
    g.push(
      el("circle", {
        cx: f(px + Math.cos(a) * 4.3),
        cy: f(py + Math.sin(a) * 4.3),
        r: 2.5,
        fill: "url(#hp)"
      })
    )
  }
  g.push(el("circle", { cx: px, cy: py, r: 5.2, fill: "url(#hp)" }))
  return svgDoc({ width: 78, height: 60, viewBox: "-8 -21 78 60" }, [
    el("defs", {}, defs),
    el("g", { filter: "url(#hs)" }, [
      el("g", { transform: "translate(0 -4) rotate(-7 28 18)" }, g)
    ])
  ])
}
