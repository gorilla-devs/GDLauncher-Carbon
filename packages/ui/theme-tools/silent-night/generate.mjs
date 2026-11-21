import fs from "node:fs"
import path from "node:path"
import { fileURLToPath, pathToFileURL } from "node:url"
import { rng } from "./rng.mjs"
import { snowCap } from "./snowCap.mjs"
import { hat } from "./hat.mjs"
import { scene } from "./scene.mjs"
import { dialogArt, dialogCap } from "./dialog.mjs"
import { frostBottomLeft, frostTopRight } from "./frost.mjs"
import { sleighBells, chime } from "./sounds.mjs"
import { VARIANTS } from "./variants.mjs"

const here = path.dirname(fileURLToPath(import.meta.url))
export const ASSETS_DIR = path.resolve(here, "../../themes/silent-night/assets")

/** Every file under `dir`, as relative paths with forward slashes. */
export function listFiles(dir, base = "") {
  if (!fs.existsSync(dir)) return []
  return fs
    .readdirSync(dir, { withFileTypes: true })
    .flatMap((e) =>
      e.isDirectory()
        ? listFiles(path.join(dir, e.name), `${base}${e.name}/`)
        : [`${base}${e.name}`]
    )
}

/** Relative path under assets/ -> file text (svg) or bytes (wav). Pure and deterministic. */
export function generateSilentNight() {
  const files = new Map()
  const s = scene()
  files.set("scene.svg", s.still)
  files.set("scene-motion.svg", s.motion)
  VARIANTS.forEach((v, i) => {
    const { seed, ...opts } = v
    files.set(
      `caps/cap-${String(i + 1).padStart(2, "0")}.svg`,
      snowCap(184, rng(seed), opts)
    )
  })
  // only 16 px of the ledge sit above the panel's top edge, hence maxH 16
  files.set(
    "ledge.svg",
    snowCap(1124, rng(311), {
      mounds: 8,
      floor: 0.28,
      thick: 15,
      icicles: 7,
      spill: "",
      maxH: 16
    })
  )
  files.set("hat.svg", hat())
  files.set("dialog-art.svg", dialogArt())
  files.set("dialog-cap.svg", dialogCap())
  files.set("frost-bl.svg", frostBottomLeft())
  files.set("frost-tr.svg", frostTopRight())
  files.set("sounds/sleigh-bells.wav", sleighBells())
  files.set("sounds/chime.wav", chime())
  return files
}

const REPO = path.resolve(here, "../../../..")
const LOGO = path.join(
  REPO,
  "apps/desktop/packages/mainWindow/public/assets/images/gdlauncher_wide_logo_blue.svg"
)

/** A contact sheet to look at the artwork by eye. */
function contactSheet(files) {
  const url = (rel) => pathToFileURL(path.join(ASSETS_DIR, rel)).href
  const caps = [...files.keys()].filter((k) => k.startsWith("caps/"))
  const tile = (rel, w) => {
    const h = Math.round(w * 0.58)
    return `<div class="tile" style="width:${w}px;height:${h}px"><img class="cap" src="${url(rel)}" style="left:-${(6 * w) / 184}px;top:-${(30 * w) / 184}px;width:${w + (12 * w) / 184}px;height:${(48 * w) / 184}px"></div>`
  }
  const sections = [
    `<h3>Caps at 184 px (tile picture is a stand-in)</h3><div class="row">${caps.map((c) => tile(c, 184)).join("")}</div>`,
    `<h3>Caps at 96 px</h3><div class="row">${caps.map((c) => tile(c, 96)).join("")}</div>`,
    `<h3>Caps at 480 px</h3><div class="row">${caps

      .map((c) => tile(c, 480))
      .join("")}</div>`,
    `<h3>Ledge at 1124 px</h3><div class="panel"><img class="ledge" src="${url("ledge.svg")}"></div>`,
    `<h3>Hat over the logo, 36 px and 108 px</h3><div class="logos">` +
      [36, 108]
        .map(
          (h) =>
            `<div class="logo" style="height:${h}px;width:${h * 5}px"><img src="${pathToFileURL(LOGO).href}" style="height:${h}px"><img class="hat" src="${url("hat.svg")}" style="left:${(-8 * h) / 64}px;top:${(-21 * h) / 64}px;width:${(78 * h) / 64}px;height:${(60 * h) / 64}px"></div>`
        )
        .join("") +
      `</div>`,
    `<h3>Dialog art and cap</h3><div style="position:relative;width:482px;height:210px;margin-top:20px"><img src="${url("dialog-art.svg")}" style="position:absolute;left:6px;top:0;width:470px;height:178px"><img src="${url("dialog-cap.svg")}" style="position:absolute;left:0;top:-16px;width:482px;height:34px"></div>`,
    `<h3>Frost corners on a dark panel</h3><div style="position:relative;width:420px;height:320px;background:#12203f"><img src="${url("frost-bl.svg")}" style="position:absolute;left:0;bottom:0"><img src="${url("frost-tr.svg")}" style="position:absolute;right:0;top:0"></div>`,
    `<h3>Scene 1600 x 986 (still), then the motion layer on top of it (frozen at its first frame)</h3><div style="position:relative;width:1600px;height:986px"><img src="${url("scene.svg")}" style="position:absolute;inset:0"><img src="${url("scene-motion.svg")}" style="position:absolute;inset:0"></div>`
  ]
  return `<!doctype html><meta charset="utf-8"><style>
body{margin:20px;background:#0c1730;color:#dfe8f8;font:14px system-ui}
h3{margin:28px 0 10px}
.row{display:flex;flex-wrap:wrap;gap:22px 16px;align-items:flex-start;padding-top:36px}
.tile{position:relative;border-radius:10px;background:linear-gradient(#5b7a52,#2f4a3a 60%,#4a3226)}
.cap{position:absolute;pointer-events:none}
.panel{position:relative;width:1124px;height:60px;margin-top:20px;border-radius:14px;background:rgba(30,50,95,.7)}
.ledge{position:absolute;left:-6px;top:-30px;width:1136px;height:48px}
.logos{display:flex;gap:60px;align-items:flex-start;padding-top:50px;background:#101c38;padding-left:20px;padding-bottom:20px}
.logo{position:relative}.logo img{display:block}.logo img.hat{position:absolute;overflow:visible}
</style>${sections.join("")}`
}

function main(argv) {
  const files = generateSilentNight()
  const sheetAt = argv.indexOf("--sheet")
  if (sheetAt >= 0) {
    const out = argv[sheetAt + 1]
    if (!out) throw new Error("--sheet needs a file path")
    fs.writeFileSync(out, contactSheet(files))
    console.log(`wrote ${out}`)
    return 0
  }
  if (argv.includes("--check")) {
    const bad = []
    for (const [rel, data] of files) {
      const p = path.join(ASSETS_DIR, rel)
      if (!fs.existsSync(p)) {
        bad.push(rel)
        continue
      }
      const same =
        typeof data === "string"
          ? fs.readFileSync(p, "utf8") === data
          : Buffer.compare(fs.readFileSync(p), Buffer.from(data)) === 0
      if (!same) bad.push(rel)
    }
    for (const rel of listFiles(ASSETS_DIR))
      if (!files.has(rel)) bad.push(`${rel} (unexpected)`)
    if (bad.length) {
      console.error(`out of date: ${bad.join(", ")}`)
      return 1
    }
    console.log("silent-night assets are up to date")
    return 0
  }
  fs.rmSync(ASSETS_DIR, { recursive: true, force: true })
  for (const [rel, data] of files) {
    const p = path.join(ASSETS_DIR, rel)
    fs.mkdirSync(path.dirname(p), { recursive: true })
    fs.writeFileSync(p, data)
  }
  console.log(`wrote ${files.size} files to ${ASSETS_DIR}`)
  return 0
}

if (
  process.argv[1] &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  process.exitCode = main(process.argv.slice(2))
}
