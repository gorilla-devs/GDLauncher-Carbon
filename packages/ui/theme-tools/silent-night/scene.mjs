import { rng } from "./rng.mjs"
import { el, f, svgDoc } from "./svg.mjs"

const W = 1600
const H = 986
const SEED = 20251224

/** What the baked vignette paints over a point (the mock's CSS gradient). */
function vignetteAlpha(x, y) {
  const t = Math.hypot((x - W * 0.45) / (W * 0.85), (y - H * 0.38) / (H * 0.75))
  return t <= 0.55 ? 0 : Math.min(1, (t - 0.55) / 0.45) * 0.45
}

const f2 = (n) => Math.round(n * 100) / 100

/** Rounds a time in seconds to a multiple of 250 ms (never "-0"). */
const q = (s) => Math.round(s * 4) / 4 + 0

/** Rounds a duration to a multiple of 500 ms, at least `min`, so each half of a
 * two-keyframe cycle is a whole number of 250 ms steps. */
const half = (s, min) => Math.max(min, Math.round(s * 2) / 2)

function defs() {
  const lg = (id, stops) =>
    el(
      "linearGradient",
      { id, x1: 0, y1: 0, x2: 0, y2: 1 },
      stops.map(([o, c, op]) =>
        el("stop", { offset: o, "stop-color": c, "stop-opacity": op })
      )
    )
  return el("defs", {}, [
    lg("sky", [
      [0, "#050c20"],
      [0.42, "#0c1b3d"],
      [0.7, "#1a325c"],
      [1, "#2c4a77"]
    ]),
    el(
      "radialGradient",
      {
        id: "moonGlow",
        cx: 985,
        cy: 215,
        r: 300,
        gradientUnits: "userSpaceOnUse"
      },
      [
        el("stop", {
          offset: 0,
          "stop-color": "#fff2d2",
          "stop-opacity": 0.42
        }),
        el("stop", {
          offset: 0.22,
          "stop-color": "#ffecc4",
          "stop-opacity": 0.14
        }),
        el("stop", { offset: 1, "stop-color": "#ffecc4", "stop-opacity": 0 })
      ]
    ),
    lg("mtnFar", [
      [0, "#7d97c2"],
      [1, "#3a5583"]
    ]),
    lg("mtnNear", [
      [0, "#3d5b8c"],
      [1, "#1d3157"]
    ]),
    lg("haze", [
      [0, "#a9c2ea", 0],
      [0.5, "#a9c2ea", 0.14],
      [1, "#a9c2ea", 0]
    ]),
    lg("snowMid", [
      [0, "#dfe8f6"],
      [1, "#9fb2d2"]
    ]),
    lg("snowFront", [
      [0, "#f7faff"],
      [1, "#c6d4ea"]
    ]),
    el("radialGradient", { id: "lampGlow" }, [
      el("stop", { offset: 0, "stop-color": "#ffd590", "stop-opacity": 0.55 }),
      el("stop", { offset: 1, "stop-color": "#ffd590", "stop-opacity": 0 })
    ]),
    el("radialGradient", { id: "winGlow" }, [
      el("stop", { offset: 0, "stop-color": "#ffc867", "stop-opacity": 0.55 }),
      el("stop", { offset: 1, "stop-color": "#ffc867", "stop-opacity": 0 })
    ]),
    // the mock's .vignette: ellipse 85% 75% at 45% 38%, transparent 55% -> .45
    el(
      "radialGradient",
      {
        id: "vignette",
        cx: 0,
        cy: 0,
        r: 1,
        gradientUnits: "userSpaceOnUse",
        gradientTransform: `translate(${f(W * 0.45)} ${f(H * 0.38)}) scale(${f(W * 0.85)} ${f(H * 0.75)})`
      },
      [
        el("stop", {
          offset: 0.55,
          "stop-color": "rgb(2 6 18)",
          "stop-opacity": 0
        }),
        el("stop", {
          offset: 1,
          "stop-color": "rgb(2 6 18)",
          "stop-opacity": 0.45
        })
      ]
    )
  ])
}

/**
 * Port of the mock's buildScene. One pass over the seeded generator, in the
 * mock's order, collects both the still picture and what moves.
 */
export function scene() {
  const r = rng(SEED)
  const stars = []
  const forest = []
  const village = []
  const twinkles = []
  const smoke = []
  const bulbs = []

  for (let i = 0; i < 170; i++) {
    const cx = f(r() * 1600)
    const cy = f(r() * 560)
    const rad = f(0.4 + Math.pow(r(), 3) * 1.4)
    const fill = r() < 0.2 ? "#cfe0ff" : "#ffffff"
    const opacity = f(0.35 + r() * 0.65)
    stars.push(el("circle", { cx, cy, r: rad, fill, opacity }))
    if (r() < 0.3) {
      const d = f(2.5 + r() * 4)
      const o = f(-r() * 5)
      twinkles.push({ cx, cy, r: rad, d: half(d, 2.5), o: q(o) })
    }
  }

  for (let x = -20; x < 1630; x += 12 + r() * 22) {
    const h = 38 + r() * 62
    const base = 806 + (r() - 0.5) * 14
    const cx = x
    const col = r() < 0.5 ? "#122539" : "#15293f"
    forest.push(
      el("rect", {
        x: f(cx - 2),
        y: f(base - 8),
        width: 4,
        height: 10,
        fill: "#0e1a28"
      })
    )
    for (let t = 0; t < 3; t++) {
      const top = base - h + t * h * 0.27
      const bot = top + h * 0.46
      const hw = h * 0.17 + t * h * 0.07
      forest.push(
        el("path", {
          d: `M${f(cx)} ${f(top)} L${f(cx - hw)} ${f(bot)} L${f(cx + hw)} ${f(bot)}Z`,
          fill: col
        })
      )
      forest.push(
        el("path", {
          d: `M${f(cx)} ${f(top)} L${f(cx - hw * 0.5)} ${f(top + (bot - top) * 0.5)} Q${f(cx)} ${f(top + (bot - top) * 0.38)} ${f(cx + hw * 0.5)} ${f(top + (bot - top) * 0.5)}Z`,
          fill: "#dfe9f7",
          opacity: 0.85
        })
      )
    }
  }

  const houses = [
    [40, 74, 52],
    [190, 60, 44],
    [420, 86, 58],
    [640, 64, 46],
    [880, 78, 54],
    [1185, 70, 50],
    [1318, 60, 42]
  ]
  houses.forEach((hs, n) => {
    const [hx, w, hh] = hs
    const by = 938
    const roofH = w * 0.42
    const chx = hx + w * 0.68
    const chy = by - hh - roofH * 0.7
    village.push(
      el("rect", {
        x: f(chx),
        y: f(chy),
        width: 9,
        height: f(roofH * 0.6),
        fill: "#3a2a2e"
      })
    )
    village.push(
      el("rect", {
        x: f(chx - 1.5),
        y: f(chy - 3),
        width: 12,
        height: 4,
        fill: "#f2f6fd"
      })
    )
    for (let sm = 0; sm < 3; sm++)
      smoke.push({
        cx: f(chx + 4.5),
        cy: f(chy - 6),
        r: 5 + sm,
        o: q(-sm * 2.3 - n * 0.7)
      })
    village.push(
      el("rect", {
        x: hx,
        y: by - hh,
        width: w,
        height: hh,
        fill: n % 2 ? "#1d2d4c" : "#22334f"
      })
    )
    village.push(
      el("rect", {
        x: hx + w * 0.5,
        y: by - hh,
        width: w * 0.5,
        height: hh,
        fill: "#000",
        opacity: 0.14
      })
    )
    village.push(
      el("path", {
        d: `M${hx - 6} ${by - hh + 2} L${hx + w / 2} ${f(by - hh - roofH)} L${hx + w + 6} ${by - hh + 2}Z`,
        fill: "#3b2a33"
      })
    )
    village.push(
      el("path", {
        d: `M${hx - 8} ${by - hh + 3} L${hx + w / 2} ${f(by - hh - roofH - 3)} L${hx + w + 8} ${by - hh + 3} L${hx + w + 4} ${by - hh + 8} Q${hx + w / 2} ${f(by - hh - roofH + 9)} ${hx - 4} ${by - hh + 8}Z`,
        fill: "#f3f7fd"
      })
    )
    const wins = w > 66 ? 2 : 1
    for (let k = 0; k < wins; k++) {
      const wx = hx + (wins === 1 ? w * 0.22 : w * (0.16 + k * 0.46))
      const wy = by - hh * 0.62
      village.push(
        el("circle", {
          cx: f(wx + 6),
          cy: f(wy + 6),
          r: 22,
          fill: "url(#winGlow)"
        })
      )
      village.push(
        el("rect", {
          x: f(wx),
          y: f(wy),
          width: 12,
          height: 12,
          rx: 1.5,
          fill: "#ffcf75"
        })
      )
      village.push(
        el("path", {
          d: `M${f(wx + 6)} ${f(wy)} v12 M${f(wx)} ${f(wy + 6)} h12`,
          stroke: "#b77a2e",
          "stroke-width": 1.2
        })
      )
    }
    village.push(
      el("rect", {
        x: f(hx + w * 0.62),
        y: f(by - hh * 0.46),
        width: 11,
        height: f(hh * 0.46),
        rx: 1,
        fill: "#5a3420"
      })
    )
    village.push(
      el("rect", {
        x: f(hx + w * 0.62 + 2),
        y: f(by - hh * 0.46 + 3),
        width: 7,
        height: 5,
        fill: "#ffc86a",
        opacity: 0.85
      })
    )
  })

  // street lamp and the lit tree, in the strip below the ad
  const lamp = [
    el("circle", { cx: 1268, cy: 880, r: 70, fill: "url(#lampGlow)" }),
    el("ellipse", {
      cx: 1268,
      cy: 944,
      rx: 46,
      ry: 7,
      fill: "#ffd590",
      opacity: 0.18
    }),
    el("rect", { x: 1266, y: 878, width: 4, height: 64, fill: "#0e1626" }),
    el("path", { d: "M1259 878 h18 l-3 -12 h-12z", fill: "#0e1626" }),
    el("rect", { x: 1263, y: 868, width: 10, height: 9, fill: "#ffe0a0" })
  ]
  const tx = 1490
  const tb = 940
  const tree = [
    el("circle", {
      cx: tx,
      cy: tb - 60,
      r: 80,
      fill: "url(#lampGlow)",
      opacity: 0.5
    }),
    el("rect", { x: tx - 4, y: tb - 12, width: 8, height: 14, fill: "#3a2416" })
  ]
  ;[
    [0, 46, 34],
    [1, 36, 30],
    [2, 26, 26],
    [3, 16, 20]
  ].forEach((tier) => {
    const tt = tb - 10 - (tier[0] + 1) * 24
    const hw = tier[1]
    const b = tt + tier[2]
    tree.push(
      el("path", {
        d: `M${tx} ${tt - 8} L${tx - hw} ${b} L${tx + hw} ${b}Z`,
        fill: "#16382c"
      })
    )
    tree.push(
      el("path", {
        d: `M${tx - hw} ${b} Q${tx} ${b - 7} ${tx + hw} ${b} L${tx + hw - 4} ${b + 3} Q${tx} ${b - 3} ${tx - hw + 4} ${b + 3}Z`,
        fill: "#eef4fc"
      })
    )
  })
  const lc = ["#ff6b5e", "#ffd46b", "#7fd8ff", "#ffffff", "#8be08f"]
  for (let l = 0; l < 26; l++) {
    const ly = tb - 14 - r() * 90
    const spread = (tb - ly) * 0.42
    const cx = f(tx + (r() - 0.5) * 2 * spread * 0.85)
    const cy = f(ly)
    const fill = lc[l % 5]
    tree.push(el("circle", { cx, cy, r: 2.2, fill }))
    const d = f(1.6 + r() * 2.4)
    const o = f(-r() * 3)
    bulbs.push({ cx, cy, fill, d: half(d, 1.5), o: q(o) })
  }
  tree.push(
    el("path", {
      d: `M${tx} ${tb - 128} l3 7 7 1 -5 5 1.5 7 -6.5 -3.5 -6.5 3.5 1.5 -7 -5 -5 7 -1z`,
      fill: "#ffd86a"
    })
  )
  tree.push(
    el("circle", {
      cx: tx,
      cy: tb - 120,
      r: 14,
      fill: "#ffd86a",
      opacity: 0.25
    })
  )

  const still = svgDoc({ width: W, height: H, viewBox: `0 0 ${W} ${H}` }, [
    defs(),
    el("rect", { width: W, height: H, fill: "url(#sky)" }),
    el("g", {}, stars),
    el("rect", { width: W, height: H, fill: "url(#moonGlow)" }),
    el("g", {}, [
      el("circle", { cx: 985, cy: 215, r: 36, fill: "#fff5de" }),
      el("circle", { cx: 973, cy: 206, r: 7, fill: "#efe2c2", opacity: 0.55 }),
      el("circle", { cx: 996, cy: 228, r: 5, fill: "#efe2c2", opacity: 0.5 }),
      el("circle", {
        cx: 1000,
        cy: 203,
        r: 3.5,
        fill: "#efe2c2",
        opacity: 0.45
      })
    ]),
    el("path", {
      d: "M0 640 L110 596 L190 618 L320 538 L420 600 L520 566 L640 612 L770 520 L880 588 L990 552 L1100 606 L1240 528 L1360 586 L1470 546 L1600 598 V986 H0Z",
      fill: "url(#mtnFar)",
      opacity: 0.55
    }),
    el("path", {
      d: "M320 538 L292 556 L306 554 L320 562 L334 552 L348 556Z M770 520 L740 540 L757 537 L770 546 L784 536 L800 541Z M1240 528 L1212 546 L1228 544 L1240 552 L1254 543 L1268 547Z",
      fill: "#e8f0fb",
      opacity: 0.6
    }),
    el("path", {
      d: "M0 704 L140 652 L250 690 L380 630 L520 702 L650 662 L800 712 L940 642 L1080 702 L1220 656 L1380 706 L1500 668 L1600 690 V986 H0Z",
      fill: "url(#mtnNear)",
      opacity: 0.9
    }),
    el("path", {
      d: "M380 630 L356 646 L370 645 L381 652 L394 644 L406 648Z M940 642 L916 658 L930 657 L941 664 L954 656 L966 660Z M1500 668 L1478 682 L1490 681 L1501 688 L1512 680 L1523 684Z",
      fill: "#dde8f7",
      opacity: 0.55
    }),
    el("rect", { y: 640, width: 1600, height: 170, fill: "url(#haze)" }),
    el("g", {}, forest),
    el("path", {
      d: "M0 820 Q200 792 420 816 T860 810 T1300 800 T1600 812 V986 H0Z",
      fill: "url(#snowMid)"
    }),
    el("g", {}, village),
    el("g", {}, lamp),
    el("g", {}, tree),
    el("path", {
      d: "M0 942 C200 916 380 952 600 934 S1000 926 1200 940 S1480 924 1600 936 V986 H0Z",
      fill: "url(#snowFront)"
    }),
    el("path", {
      d: "M0 958 C240 948 420 968 700 956 S1150 950 1600 962",
      stroke: "#b8c8e2",
      "stroke-width": 2,
      fill: "none",
      opacity: 0.5
    }),
    el("rect", { width: W, height: H, fill: "url(#vignette)" })
  ])

  // ---- the motion layer: every animation steps in whole 250 ms ----
  // A two-keyframe cycle takes steps(2 * seconds) per half, so it changes at
  // most 4 times a second. Smoke has keyframes at 0, 1 s and 7 s of its 7 s
  // cycle, which are 4 and 24 steps long. The motion layer is not under the
  // vignette, so each element's peak opacity (--p) is already multiplied by
  // what the vignette leaves at its position.
  const css = [
    "@keyframes gdTw{0%,100%{opacity:0}50%{opacity:var(--p)}}",
    "@keyframes gdBulb{0%,100%{opacity:0}50%{opacity:var(--p)}}",
    "@keyframes gdSmoke{0%{transform:translate(0,0) scale(.6);opacity:0;animation-timing-function:steps(4,end)}14.2857%{transform:translate(2px,-6.6px) scale(.74);opacity:var(--p);animation-timing-function:steps(24,end)}100%{transform:translate(14px,-46px) scale(1.6);opacity:0}}",
    ".sm{transform-box:fill-box}",
    "@media (prefers-reduced-motion:reduce){*{animation:none!important}}"
  ].join("")
  const anim = (name, dur, delay, peak) =>
    `--p:${peak};animation:${name} ${dur}s steps(${dur * 2},end) ${delay}s infinite`
  const keep = (x, y) => 1 - vignetteAlpha(x, y)
  const motion = [
    el("style", {}, css),
    el(
      "g",
      {},
      twinkles.map((s) =>
        el("circle", {
          cx: s.cx,
          cy: s.cy,
          r: f(s.r + 0.5),
          fill: "#ffffff",
          opacity: 0,
          style: anim("gdTw", s.d, s.o, f2(0.6 * keep(s.cx, s.cy)))
        })
      )
    ),
    el(
      "g",
      {},
      smoke.map((s) =>
        el("circle", {
          cx: s.cx,
          cy: s.cy,
          r: s.r,
          fill: "#c9d4e6",
          opacity: 0,
          class: "sm",
          // the keyframes carry the real step lengths; this is the fallback
          style: `--p:${f2(0.5 * keep(s.cx, s.cy))};animation:gdSmoke 7s steps(4,end) ${s.o}s infinite`
        })
      )
    ),
    el(
      "g",
      {},
      bulbs.map((b) =>
        el("circle", {
          cx: b.cx,
          cy: b.cy,
          r: 3.25,
          fill: b.fill,
          opacity: 0,
          style: anim("gdBulb", b.d, b.o, f2(0.35 * keep(b.cx, b.cy)))
        })
      )
    )
  ]
  return {
    still,
    motion: svgDoc({ width: W, height: H, viewBox: `0 0 ${W} ${H}` }, motion)
  }
}
