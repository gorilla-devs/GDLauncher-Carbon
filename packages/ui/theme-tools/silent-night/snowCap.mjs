import { el, f, svgDoc } from "./svg.mjs"

const PAD = 6
const TOP = 30
const VH = 48

/**
 * A snow cap sitting on the top edge of a box (port of the mock's snowCap).
 * Every call gets its own shape from `rand`; `opts` pins parts of it.
 *
 * Changes from the mock: the mound height is soft-clamped to `maxH` (19 by
 * default, so the cap never reaches the row above a 184 px tile), the glints
 * are static, and a contact shadow is baked in.
 *
 * Returns a whole <svg> document.
 */
export function snowCap(W, r, o = {}) {
  const VW = W + PAD * 2
  const maxH = o.maxH != null ? o.maxH : 19
  const knee = maxH * 0.7
  const clampH = (v) =>
    v <= knee
      ? v
      : Math.min(
          maxH,
          knee + (maxH - knee) * Math.tanh((v - knee) / (maxH - knee))
        )

  r() // the mock spends one draw on a random element id; ids are fixed here

  const defs = [
    el("linearGradient", { id: "sf", x1: 0, y1: 0, x2: 0, y2: 1 }, [
      el("stop", { offset: 0, "stop-color": "#ffffff" }),
      el("stop", { offset: 0.45, "stop-color": "#f4f8fe" }),
      el("stop", { offset: 0.82, "stop-color": "#d3dff1" }),
      el("stop", { offset: 1, "stop-color": "#afc3e2" })
    ]),
    el("linearGradient", { id: "ic", x1: 0, y1: 0, x2: 0, y2: 1 }, [
      el("stop", {
        offset: 0,
        "stop-color": "#f6fbff",
        "stop-opacity": 0.95
      }),
      el("stop", { offset: 1, "stop-color": "#bcd6f2", "stop-opacity": 0.7 })
    ]),
    el("linearGradient", { id: "sh", x1: 0, y1: 0, x2: 0, y2: 1 }, [
      el("stop", {
        offset: 0,
        "stop-color": "rgb(4 12 34)",
        "stop-opacity": 0.38
      }),
      el("stop", { offset: 1, "stop-color": "rgb(4 12 34)", "stop-opacity": 0 })
    ]),
    el(
      "filter",
      {
        id: "ds",
        filterUnits: "userSpaceOnUse",
        x: 0,
        y: 0,
        width: VW,
        height: VH
      },
      el("feDropShadow", {
        dx: 0,
        dy: 2,
        stdDeviation: 1.5,
        "flood-color": "#000c28",
        "flood-opacity": 0.38
      })
    )
  ]

  const pick = r()
  const T =
    o.thick != null
      ? o.thick
      : pick < 0.15
        ? 4 + r() * 2.5
        : pick < 0.55
          ? 10 + r() * 6
          : 17 + r() * 8
  const floor = o.floor != null ? o.floor : 0.08 + r() * 0.4
  const K = o.mounds || 1 + Math.floor(r() * 4)
  const mounds = []
  for (let m = 0; m < K; m++)
    mounds.push([r(), 0.05 + r() * 0.17, 0.3 + r() * 0.7])
  const p1 = r() * 6.28
  const p2 = r() * 6.28
  const p3 = r() * 6.28
  const f1 = 7 + r() * 6
  const f2 = 19 + r() * 10
  const raw = (u) => {
    let s = 0
    for (const q of mounds)
      s += q[2] * Math.exp(-Math.pow((u - q[0]) / q[1], 2))
    return s
  }
  let mx = 0
  for (let i = 0; i <= 60; i++) mx = Math.max(mx, raw(i / 60))
  const h = (u) => {
    const e = floor + ((1 - floor) * raw(u)) / (mx || 1)
    const n =
      1 +
      0.14 * Math.sin(u * f1 + p1) +
      0.1 * Math.sin(u * f2 + p2) +
      0.07 * Math.sin(u * 53 + p3)
    const corner = Math.min(1, Math.min(u, 1 - u) / 0.045)
    return Math.max(1.2, clampH(T * e * n * (0.35 + 0.65 * Math.sqrt(corner))))
  }
  const N = Math.max(30, Math.round(W / 5))
  const pts = []
  for (let j = 0; j <= N; j++) {
    const u = j / N
    pts.push([PAD + u * W, TOP - h(u)])
  }
  const smooth = (P) => {
    let d = ""
    for (let k = 0; k < P.length - 1; k++) {
      const a = P[Math.max(0, k - 1)]
      const b = P[k]
      const c = P[k + 1]
      const e2 = P[Math.min(P.length - 1, k + 2)]
      d +=
        " C" +
        f(b[0] + (c[0] - a[0]) / 6) +
        " " +
        f(b[1] + (c[1] - a[1]) / 6) +
        " " +
        f(c[0] - (e2[0] - b[0]) / 6) +
        " " +
        f(c[1] - (e2[1] - b[1]) / 6) +
        " " +
        f(c[0]) +
        " " +
        f(c[1])
    }
    return d
  }
  const spill =
    o.spill != null ? o.spill : r() < 0.55 ? (r() < 0.5 ? "l" : "r") : ""
  const sl = 6 + r() * 9
  const L0 = pts[0]
  const R0 = pts[pts.length - 1]
  let d = ""
  if (spill === "l")
    d +=
      "M" +
      (PAD + 4) +
      " " +
      (TOP + 2) +
      " C" +
      (PAD - 2.5) +
      " " +
      (TOP + 2) +
      " " +
      (PAD - 3) +
      " " +
      f(TOP + sl) +
      " " +
      (PAD - 1) +
      " " +
      f(TOP + sl) +
      " C" +
      (PAD - 4.5) +
      " " +
      f(TOP + sl * 0.4) +
      " " +
      (PAD - 3.5) +
      " " +
      f(L0[1]) +
      " " +
      f(L0[0]) +
      " " +
      f(L0[1])
  else
    d +=
      "M" +
      PAD +
      " " +
      (TOP + 2) +
      " C" +
      (PAD - 1.8) +
      " " +
      (TOP + 1) +
      " " +
      (PAD - 1.5) +
      " " +
      f(L0[1] + 1) +
      " " +
      f(L0[0]) +
      " " +
      f(L0[1])
  d += smooth(pts)
  if (spill === "r")
    d +=
      " C" +
      (PAD + W + 3.5) +
      " " +
      f(R0[1]) +
      " " +
      (PAD + W + 4.5) +
      " " +
      f(TOP + sl * 0.4) +
      " " +
      (PAD + W + 1) +
      " " +
      f(TOP + sl) +
      " C" +
      (PAD + W + 3) +
      " " +
      f(TOP + sl) +
      " " +
      (PAD + W + 2.5) +
      " " +
      (TOP + 2) +
      " " +
      (PAD + W - 4) +
      " " +
      (TOP + 2)
  else
    d +=
      " C" +
      (PAD + W + 1.5) +
      " " +
      f(R0[1] + 1) +
      " " +
      (PAD + W + 1.8) +
      " " +
      (TOP + 1) +
      " " +
      (PAD + W) +
      " " +
      (TOP + 2)
  for (let s = 7; s >= 0; s--)
    d +=
      " L" +
      f(PAD + (W * s) / 8) +
      " " +
      f(TOP + 1.6 + Math.sin(s * 1.7 + p1) * 0.9)
  d += " Z"

  const snow = []
  snow.push(el("path", { d, fill: "url(#sf)" }))
  snow.push(
    el("path", {
      d: "M" + PAD + " " + (TOP + 1.4) + " L" + (PAD + W) + " " + (TOP + 1.4),
      stroke: "#93aad0",
      "stroke-width": 1.4,
      opacity: 0.65
    })
  )
  snow.push(
    el("path", {
      d:
        "M" +
        f(L0[0]) +
        " " +
        f(L0[1] + 0.8) +
        smooth(pts.map((p) => [p[0], p[1] + 0.8])),
      stroke: "#ffffff",
      "stroke-width": 1.3,
      fill: "none",
      opacity: 0.95,
      "stroke-linecap": "round"
    })
  )
  const clumps = Math.floor(r() * 3)
  for (let c = 0; c < clumps; c++) {
    const cu = 0.1 + r() * 0.8
    const cr = 1.5 + r() * 2.3
    snow.push(
      el("ellipse", {
        cx: f(PAD + cu * W),
        cy: f(TOP - Math.min(h(cu), maxH - 0.2 - cr * 0.65) + cr * 0.35),
        rx: f(cr * 1.5),
        ry: f(cr),
        fill: "#ffffff"
      })
    )
  }
  const ic = o.icicles != null ? o.icicles : Math.floor(r() * r() * 5)
  for (let q = 0; q < ic; q++) {
    const ix = PAD + 8 + r() * (W - 16)
    const il = 3 + r() * 11
    const iw = 1.8 + r() * 2.2
    snow.push(
      el("path", {
        d:
          "M" +
          f(ix - iw) +
          " " +
          (TOP + 1) +
          " Q" +
          f(ix - iw * 0.35) +
          " " +
          f(TOP + il * 0.6) +
          " " +
          f(ix + (r() - 0.5)) +
          " " +
          f(TOP + 1 + il) +
          " Q" +
          f(ix + iw * 0.35) +
          " " +
          f(TOP + il * 0.6) +
          " " +
          f(ix + iw) +
          " " +
          (TOP + 1) +
          "Z",
        fill: "url(#ic)"
      })
    )
    snow.push(
      el("path", {
        d:
          "M" +
          f(ix - iw * 0.35) +
          " " +
          (TOP + 2) +
          " L" +
          f(ix - 0.2) +
          " " +
          f(TOP + il * 0.75),
        stroke: "#ffffff",
        "stroke-width": 0.7,
        opacity: 0.8
      })
    )
  }
  const gl = o.glints != null ? o.glints : 1 + Math.floor(r() * 3)
  for (let z = 0; z < gl; z++) {
    const gu = 0.08 + r() * 0.84
    const gx = PAD + gu * W
    const gyRaw = h(gu) * (0.3 + r() * 0.5)
    const gs = 1.5 + r() * 1.5
    const gy = TOP - Math.min(gyRaw, maxH - 0.2 - gs * 2)
    snow.push(
      el("path", {
        fill: "#ffffff",
        opacity: 0.9,
        d:
          "M" +
          f(gx) +
          " " +
          f(gy - gs * 2) +
          " L" +
          f(gx + gs * 0.4) +
          " " +
          f(gy - gs * 0.4) +
          " L" +
          f(gx + gs * 2) +
          " " +
          f(gy) +
          " L" +
          f(gx + gs * 0.4) +
          " " +
          f(gy + gs * 0.4) +
          " L" +
          f(gx) +
          " " +
          f(gy + gs * 2) +
          " L" +
          f(gx - gs * 0.4) +
          " " +
          f(gy + gs * 0.4) +
          " L" +
          f(gx - gs * 2) +
          " " +
          f(gy) +
          " L" +
          f(gx - gs * 0.4) +
          " " +
          f(gy - gs * 0.4) +
          "Z"
      })
    )
  }

  const body = [
    el("defs", {}, defs),
    el("rect", {
      x: PAD,
      y: TOP + 1.6,
      width: W,
      height: 16,
      fill: "url(#sh)"
    }),
    el("g", { filter: "url(#ds)" }, snow)
  ]
  return svgDoc(
    {
      width: VW,
      height: VH,
      viewBox: `0 0 ${VW} ${VH}`,
      preserveAspectRatio: "none"
    },
    body
  )
}
