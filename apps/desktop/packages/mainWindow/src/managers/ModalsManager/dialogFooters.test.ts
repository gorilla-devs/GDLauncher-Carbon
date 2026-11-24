import { describe, expect, it } from "vitest"

const sources = import.meta.glob<string>("./modals/**/*.tsx", {
  query: "?raw",
  import: "default",
  eager: true
})

/** Back-out keys: Cancel, or Later / Never / Anyway style choices. */
const BACK_OUT = /_trn_[\w.]*(cancel|later|never|anyway)\b/

/**
 * Boxed buttons that legitimately carry a back-out looking key because they
 * act on something in the dialog body, not on the dialog. Format: "file::key".
 */
const ALLOWED = new Set<string>([
  // Cancels the unfinished install the dialog describes, not the dialog.
  "./modals/RepairModpack/index.tsx::instances:_trn_repair_pending_cancel"
])

/** Every `<Button` ... `</Button>` block in a source. */
const buttonBlocks = (source: string) => {
  const blocks: string[] = []
  let from = 0
  for (;;) {
    const start = source.indexOf("<Button", from)
    if (start === -1) return blocks
    // `<ButtonGroup` and friends are not buttons.
    if (/\w/.test(source[start + "<Button".length] ?? "")) {
      from = start + 1
      continue
    }
    const end = source.indexOf("</Button>", start)
    if (end === -1) return blocks
    blocks.push(source.slice(start, end + "</Button>".length))
    from = end
  }
}

const keysIn = (block: string) =>
  [...block.matchAll(/[\w:]*_trn_[\w.]+/g)].map((m) => m[0])

describe("dialog footers", () => {
  it("no dialog shows a back-out choice as a boxed button", () => {
    const offenders: string[] = []
    for (const [file, source] of Object.entries(sources)) {
      for (const block of buttonBlocks(source)) {
        for (const key of keysIn(block)) {
          if (key !== "general:_trn_cancel" && !BACK_OUT.test(key)) continue
          if (ALLOWED.has(`${file}::${key}`)) continue
          offenders.push(`${file} shows "${key}" in a <Button>`)
        }
      }
    }
    expect(
      offenders,
      `Back-out choices must be a TextLink or CancelLink:\n${offenders.join("\n")}`
    ).toEqual([])
  })

  it("the guard sees the dialogs", () => {
    const files = Object.keys(sources)
    expect(files.length).toBeGreaterThan(40)
    expect(files.some((f) => sources[f].includes("CancelLink"))).toBe(true)
  })

  it("does not mistake wizard Back buttons for back-out choices", () => {
    for (const key of [
      "onboarding:_trn_prev",
      "onboarding:_trn_go_back",
      "content:_trn_duplicates.summary.button_back"
    ]) {
      expect(BACK_OUT.test(key)).toBe(false)
    }
  })
})
