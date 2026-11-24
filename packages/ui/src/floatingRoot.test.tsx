import { afterEach, describe, expect, it } from "vitest"
import { render, screen, cleanup } from "solid-testing-library"
import { getFloatingRoot } from "./floatingRoot"
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent
} from "./DropdownMenu"

afterEach(() => {
  cleanup()
  document.getElementById("floating-root")?.remove()
})

describe("getFloatingRoot", () => {
  it("falls back to body", () => {
    expect(getFloatingRoot()).toBe(document.body)
  })

  it("uses #floating-root when present", () => {
    const el = document.createElement("div")
    el.id = "floating-root"
    document.body.appendChild(el)
    expect(getFloatingRoot()).toBe(el)
  })

  it("DropdownMenu content mounts inside the floating root", () => {
    const el = document.createElement("div")
    el.id = "floating-root"
    document.body.appendChild(el)

    render(() => (
      <DropdownMenu open>
        <DropdownMenuTrigger>open</DropdownMenuTrigger>
        <DropdownMenuContent>
          <div data-testid="item">x</div>
        </DropdownMenuContent>
      </DropdownMenu>
    ))

    expect(
      document
        .getElementById("floating-root")!
        .contains(screen.getByTestId("item"))
    ).toBe(true)
  })
})
