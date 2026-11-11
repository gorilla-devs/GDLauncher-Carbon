import { afterEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"
import { createGenerator, presetWind3 } from "unocss"
import { Checkbox } from "./index"

let dispose: (() => void) | undefined
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
})

function mount(component: () => any) {
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(component, root)
  return root.querySelector("label")!
}

// Generates the CSS the app's UnoCSS preset makes for these classes and
// returns each rule as its selector and declarations.
async function rules(tokens: string[]) {
  const uno = await createGenerator({
    presets: [presetWind3()],
    theme: { colors: { primary: { 400: "#5078ff" } } }
  })
  const { css } = await uno.generate(tokens.join(" "), {
    preflights: false,
    safelist: false
  })
  return Array.from(
    css.replace(/\/\*[\s\S]*?\*\//g, "").matchAll(/([^{}]+)\{([^{}]*)\}/g),
    ([, selector, body]) => ({ selector: selector.trim(), body })
  )
}

it("draws its focus ring only for keyboard focus, not after a mouse click", async () => {
  const label = mount(() => <Checkbox onChange={vi.fn()} ariaLabel="A" />)
  const tokens = Array.from(label.classList)

  // A mouse click on the label also focuses the input, so a ring keyed on
  // :focus-within or :focus would stay after every click.
  expect(tokens).toContain("has-[:focus-visible]:ring-2")
  expect(tokens).toContain("has-[:focus-visible]:ring-primary-400")
  expect(
    tokens.filter(
      (t) => t.startsWith("focus-within:") || t.startsWith("focus:")
    )
  ).toEqual([])

  const ring = await rules(tokens.filter((t) => t.includes("ring")))
  expect(ring.some((r) => r.body.includes("--un-ring-width:2px"))).toBe(true)
  expect(ring.some((r) => r.body.includes("--un-ring-color"))).toBe(true)
  for (const rule of ring) {
    expect(rule.selector).toContain(":has(:focus-visible)")
    expect(rule.selector).not.toContain(":focus-within")
  }
})
