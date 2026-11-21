import { rng } from "./rng.mjs"
import { el, f, svgDoc } from "./svg.mjs"

/** The seasonal dialog's picture (470 x 178, seed 1224), static. */
export function dialogArt() {
  const r = rng(1224)
  const stars = []
  for (let i = 0; i < 50; i++) {
    const cx = f(r() * 470)
    const cy = f(r() * 100)
    const rad = f(0.4 + r() * 0.9)
    const opacity = f(0.4 + r() * 0.6)
    stars.push(el("circle", { cx, cy, r: rad, fill: "#fff", opacity }))
    // the mock twinkled 30% of them and spent two more draws on it
    if (r() < 0.3) r()
  }
  const forest = []
  for (let x = -10; x < 480; x += 10 + r() * 16) {
    const h = 22 + r() * 30
    const b = 146
    const cx = x
    for (let t = 0; t < 3; t++) {
      const top = b - h + t * h * 0.27
      const bot = top + h * 0.46
      const hw = h * 0.17 + t * h * 0.07
      forest.push(
        el("path", {
          d: `M${f(cx)} ${f(top)} L${f(cx - hw)} ${f(bot)} L${f(cx + hw)} ${f(bot)}Z`,
          fill: "#13263d"
        })
      )
      forest.push(
        el("path", {
          d: `M${f(cx)} ${f(top)} L${f(cx - hw * 0.5)} ${f(top + (bot - top) * 0.5)} L${f(cx + hw * 0.5)} ${f(top + (bot - top) * 0.5)}Z`,
          fill: "#dfe9f7",
          opacity: 0.8
        })
      )
    }
  }
  const village = []
  ;[
    [40, 44, 30],
    [120, 36, 26],
    [300, 50, 34],
    [380, 38, 26]
  ].forEach(([hx, w, hh]) => {
    const by = 166
    const rh = w * 0.42
    village.push(
      el("rect", { x: hx, y: by - hh, width: w, height: hh, fill: "#1d2d4c" })
    )
    village.push(
      el("path", {
        d: `M${hx - 5} ${by - hh + 2} L${hx + w / 2} ${f(by - hh - rh)} L${hx + w + 5} ${by - hh + 2} L${hx + w + 2} ${by - hh + 6} Q${hx + w / 2} ${f(by - hh - rh + 7)} ${hx - 2} ${by - hh + 6}Z`,
        fill: "#f3f7fd"
      })
    )
    village.push(
      el("circle", {
        cx: f(hx + w * 0.3 + 4),
        cy: f(by - hh * 0.55 + 4),
        r: 14,
        fill: "url(#dwin)"
      })
    )
    village.push(
      el("rect", {
        x: f(hx + w * 0.3),
        y: f(by - hh * 0.55),
        width: 8,
        height: 8,
        rx: 1,
        fill: "#ffcf75"
      })
    )
  })
  return svgDoc(
    {
      width: 470,
      height: 178,
      viewBox: "0 0 470 178",
      preserveAspectRatio: "xMidYMax slice"
    },
    [
      el("defs", {}, [
        el("linearGradient", { id: "dsky", x1: 0, y1: 0, x2: 0, y2: 1 }, [
          el("stop", { offset: 0, "stop-color": "#060d22" }),
          el("stop", { offset: 0.6, "stop-color": "#16295a" }),
          el("stop", { offset: 1, "stop-color": "#2c4a77" })
        ]),
        el(
          "radialGradient",
          {
            id: "dmoon",
            cx: 360,
            cy: 52,
            r: 90,
            gradientUnits: "userSpaceOnUse"
          },
          [
            el("stop", {
              offset: 0,
              "stop-color": "#fff2d2",
              "stop-opacity": 0.45
            }),
            el("stop", {
              offset: 1,
              "stop-color": "#fff2d2",
              "stop-opacity": 0
            })
          ]
        ),
        el("radialGradient", { id: "dwin" }, [
          el("stop", {
            offset: 0,
            "stop-color": "#ffc867",
            "stop-opacity": 0.6
          }),
          el("stop", { offset: 1, "stop-color": "#ffc867", "stop-opacity": 0 })
        ])
      ]),
      el("rect", { width: 470, height: 178, fill: "url(#dsky)" }),
      el("g", {}, stars),
      el("rect", { width: 470, height: 178, fill: "url(#dmoon)" }),
      el("circle", { cx: 360, cy: 52, r: 17, fill: "#fff5de" }),
      el("path", {
        d: "M0 120 L60 96 L110 108 L170 82 L230 112 L290 92 L350 116 L410 88 L470 104 V178 H0Z",
        fill: "#3a5583",
        opacity: 0.7
      }),
      el("path", {
        d: "M170 82 L158 90 L165 89 L171 94 L178 88 L184 90Z M410 88 L399 96 L406 95 L411 99 L417 94 L423 96Z",
        fill: "#e8f0fb",
        opacity: 0.6
      }),
      el("g", {}, forest),
      el("path", {
        d: "M0 150 Q120 136 240 148 T470 142 V178 H0Z",
        fill: "#e6edf8"
      }),
      el("g", {}, village),
      el("path", {
        d: "M0 166 C120 158 220 172 330 164 S430 160 470 166 V178 H0Z",
        fill: "#f7faff"
      })
    ]
  )
}

/** The dialog's own snow cap (482 x 34, the extra 4 px hold the baked shadow), the popup mock's version. */
export function dialogCap() {
  const W = 470
  const PAD = 6
  const TOP = 18
  const mounds = [
    [0.18, 0.12, 1],
    [0.62, 0.16, 0.8],
    [0.86, 0.07, 0.6]
  ]
  const pts = []
  for (let j = 0; j <= 90; j++) {
    const u = j / 90
    let s = 0.25
    for (const m of mounds)
      s += m[2] * Math.exp(-Math.pow((u - m[0]) / m[1], 2))
    const corner = Math.min(1, Math.min(u, 1 - u) / 0.03)
    pts.push([
      PAD + u * W,
      TOP -
        Math.max(
          1.2,
          11 *
            Math.min(1.2, s) *
            (1 + 0.1 * Math.sin(u * 31)) *
            (0.35 + 0.65 * Math.sqrt(corner))
        )
    ])
  }
  let d = `M${PAD} ${TOP + 2} L${f(pts[0][0])} ${f(pts[0][1])}`
  for (let k = 1; k < pts.length; k++) d += ` L${f(pts[k][0])} ${f(pts[k][1])}`
  d += ` L${PAD + W} ${TOP + 2} Z`
  const parts = [
    el(
      "defs",
      {},
      el("linearGradient", { id: "dcap", x1: 0, y1: 0, x2: 0, y2: 1 }, [
        el("stop", { offset: 0, "stop-color": "#fff" }),
        el("stop", { offset: 0.7, "stop-color": "#eef3fb" }),
        el("stop", { offset: 1, "stop-color": "#b9cbe6" })
      ])
    ),
    el("path", { d, fill: "url(#dcap)" })
  ]
  const shadow = el(
    "filter",
    {
      id: "dsh",
      filterUnits: "userSpaceOnUse",
      x: 0,
      y: 0,
      width: 482,
      height: 34
    },
    el("feDropShadow", {
      dx: 0,
      dy: 2,
      stdDeviation: 0.75,
      "flood-color": "rgb(0 12 40)",
      "flood-opacity": 0.4
    })
  )
  ;[
    [120, 7],
    [300, 10],
    [410, 5]
  ].forEach(([x, l]) =>
    parts.push(
      el("path", {
        d: `M${x - 2.5} ${TOP + 1} Q${x - 1} ${f(TOP + l * 0.6)} ${x} ${TOP + 1 + l} Q${x + 1} ${f(TOP + l * 0.6)} ${x + 2.5} ${TOP + 1}Z`,
        fill: "#e6f1ff",
        opacity: 0.9
      })
    )
  )
  const [defsPart, ...shapes] = parts
  return svgDoc(
    {
      width: 482,
      height: 34,
      viewBox: "0 0 482 34",
      preserveAspectRatio: "none"
    },
    [
      el("defs", {}, [defsPart.replace(/^<defs>|<\/defs>$/g, ""), shadow]),
      el("g", { filter: "url(#dsh)" }, shapes)
    ]
  )
}
