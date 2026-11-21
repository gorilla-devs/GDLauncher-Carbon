import { createGenerator } from "unocss"
import { unocssConfig } from "@gd/config"
import { it, expect } from "vitest"

it("maps rounded-* to the radius scale", async () => {
  const uno = await createGenerator(unocssConfig as never)
  // safelist off: the config's safelist would pull in every flag icon collection
  const { css } = await uno.generate(
    "rounded rounded-md rounded-full rounded-t-xl rounded-none",
    { preflights: false, safelist: false }
  )
  expect(css).toContain("calc(0.25rem * var(--radius-scale, 1))")
  expect(css).toContain("calc(0.375rem * var(--radius-scale, 1))")
  expect(css).toContain("calc(9999px * var(--radius-scale, 1))")
  expect(css).toMatch(
    /border-top-left-radius:calc\(0\.75rem \* var\(--radius-scale, 1\)\)/
  )
  expect(css).toMatch(/\.rounded-none\{border-radius:0;\}/)
})
