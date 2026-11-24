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
  return root
}

// Generates the CSS the app's UnoCSS preset makes for these classes and
// returns each rule as its selector and declarations.
async function rules(tokens: string[]) {
  const uno = await createGenerator({
    presets: [presetWind3()],
    theme: { colors: { primary: { 500: "#5078ff" } } }
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
  const root = mount(() => <Checkbox onChange={vi.fn()} ariaLabel="A" />)
  const input = root.querySelector("input")!
  const box = root.querySelector<HTMLElement>('[data-gdl="checkbox.box"]')!
  const tokens = Array.from(box.classList)

  // The ring is the box's outline, drawn while the real checkbox before it
  // has keyboard focus.
  expect(input.classList.contains("peer")).toBe(true)
  expect(input.nextElementSibling).toBe(box)
  expect(tokens).toContain("peer-focus-visible:outline")
  expect(tokens).toContain("peer-focus-visible:outline-2")
  expect(tokens).toContain("peer-focus-visible:outline-primary-500")
  // A ring keyed on :focus-within or :focus would stay after a click.
  for (const element of [box, box.parentElement!]) {
    expect(
      Array.from(element.classList).filter(
        (t) =>
          t.startsWith("focus-within:") ||
          t.startsWith("focus:") ||
          t.startsWith("peer-focus:")
      )
    ).toEqual([])
  }

  const ring = await rules(tokens.filter((t) => t.includes("outline")))
  expect(ring.some((r) => r.body.includes("outline-width:2px"))).toBe(true)
  expect(ring.some((r) => r.body.includes("outline-color"))).toBe(true)
  for (const rule of ring) {
    expect(rule.selector).toContain(".peer:focus-visible")
    expect(rule.selector).not.toContain(":focus-within")
  }
})
