import { afterEach, describe, expect, it } from "vitest"
import { render, screen, cleanup } from "solid-testing-library"
import { createSignal, Show } from "solid-js"
import { Button } from "./Button"
import { Badge } from "./Badge"
import { Input } from "./Input"
import { Switch } from "./Switch"
import { Tabs, TabsList, TabsTrigger } from "./Tabs"
import { Tooltip, TooltipContent, TooltipTrigger } from "./Tooltip"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuTrigger
} from "./DropdownMenu"
import { Toaster, toast } from "./Sonner"
import { Collapsable } from "./Collapsable"
import { gdlHooks } from "./hooks"

afterEach(() => {
  cleanup()
  document.body.innerHTML = ""
})

describe("gdlHooks", () => {
  it("appends string extras only", () => {
    expect(gdlHooks("a")).toBe("a")
    expect(gdlHooks("a", "b c")).toBe("a b c")
    expect(gdlHooks("a", 3)).toBe("a")
  })
})

describe("data-gdl hooks", () => {
  it("buttons carry their type", () => {
    render(() => (
      <>
        <Button type="secondary">two</Button>
        <Button>one</Button>
      </>
    ))
    expect(screen.getByText("two").getAttribute("data-gdl")).toBe(
      "button button.secondary"
    )
    expect(screen.getByText("one").getAttribute("data-gdl")).toBe(
      "button button.primary"
    )
  })

  it("buttons flag a tinted or disabled look, not a loading one", () => {
    render(() => (
      <>
        <Button variant="red">tinted</Button>
        <Button backgroundColor="bg-blue-500">custom</Button>
        <Button disabled>off</Button>
        <Button loading data-testid="busy">
          busy
        </Button>
      </>
    ))
    const hooks = (t: string) =>
      screen.getByText(t).getAttribute("data-gdl")!.split(" ")
    const busy = screen.getByTestId("busy").getAttribute("data-gdl")!.split(" ")
    expect(hooks("tinted")).toContain("button.tinted")
    expect(hooks("tinted")).not.toContain("button.disabled")
    expect(hooks("custom")).toContain("button.tinted")
    expect(hooks("off")).toContain("button.disabled")
    expect(hooks("off")).not.toContain("button.tinted")
    expect(busy).not.toContain("button.disabled")
    expect(busy).not.toContain("button.tinted")
  })

  it("badges are gold only while they keep bg-primary-500", () => {
    render(() => (
      <>
        <Badge>gold</Badge>
        <Badge class="bg-darkSlate-600">chip</Badge>
        <Badge variant="success">ok</Badge>
      </>
    ))
    const hooks = (t: string) =>
      screen.getByText(t).getAttribute("data-gdl")!.split(" ")
    expect(hooks("gold")).toEqual(["badge", "badge.default"])
    expect(hooks("chip")).toEqual(["badge"])
    expect(hooks("ok")).toEqual(["badge"])
  })

  it("the inner input field has its own hook", () => {
    const { container } = render(() => <Input />)
    expect(
      container.querySelector('input[data-gdl="input.field"]')
    ).not.toBeNull()
  })

  it("a toast's action button has a hook", async () => {
    render(() => <Toaster />)
    toast("with action", { action: { label: "Go", onClick: () => {} } })
    await screen.findByText("Go")
    await new Promise<void>((resolve, reject) => {
      const start = Date.now()
      const tick = () => {
        if (document.querySelector('[data-action][data-gdl="toast.action"]'))
          return resolve()
        if (Date.now() - start > 3000)
          return reject(new Error("no toast.action hook"))
        setTimeout(tick, 20)
      }
      tick()
    })
  })

  it("incoming hooks are merged", () => {
    const { container } = render(() => <Input data-gdl="library.search" />)
    const el = container.querySelector('[data-gdl="input library.search"]')
    expect(el).not.toBeNull()
    expect(el!.querySelector("input")).not.toBeNull()
  })

  it("the active tab is marked", () => {
    const [value, setValue] = createSignal("a")
    render(() => (
      <Tabs value={value()}>
        <TabsList>
          <TabsTrigger value="a">A</TabsTrigger>
          <TabsTrigger value="b">B</TabsTrigger>
        </TabsList>
      </Tabs>
    ))
    const a = screen.getByText("A")
    const b = screen.getByText("B")
    expect(a.getAttribute("data-gdl")).toBe("tab tab.active")
    expect(b.getAttribute("data-gdl")).toBe("tab")
    setValue("b")
    expect(a.getAttribute("data-gdl")).toBe("tab")
    expect(b.getAttribute("data-gdl")).toBe("tab tab.active")
  })

  it("switch, tooltip, dropdown and toast carry hooks", async () => {
    const { container } = render(() => (
      <>
        <Switch />
        <Tooltip open>
          <TooltipTrigger>t</TooltipTrigger>
          <TooltipContent>tip</TooltipContent>
        </Tooltip>
        <DropdownMenu open>
          <DropdownMenuTrigger>d</DropdownMenuTrigger>
          <DropdownMenuContent>menu</DropdownMenuContent>
        </DropdownMenu>
        <Toaster />
      </>
    ))
    expect(container.querySelector('[data-gdl~="switch"]')).not.toBeNull()
    expect(document.querySelector('[data-gdl~="tooltip"]')).not.toBeNull()
    expect(document.querySelector('[data-gdl~="dropdown"]')).not.toBeNull()
    toast("x")
    await screen.findByText("x")
    await new Promise<void>((resolve, reject) => {
      const start = Date.now()
      const tick = () => {
        if (document.querySelector('[data-somoto-toast][data-gdl="toast"]'))
          return resolve()
        if (Date.now() - start > 3000) return reject(new Error("no toast hook"))
        setTimeout(tick, 20)
      }
      tick()
    })
  })

  it("a collapsable adds headerGdl to its default header row only when given", () => {
    const { container } = render(() => (
      <>
        <Collapsable title="with" headerGdl="section-title">
          body
        </Collapsable>
        <Collapsable title="without">body</Collapsable>
      </>
    ))
    const rows = [...container.querySelectorAll("button.cursor-pointer")]
    expect(rows).toHaveLength(2)
    expect(rows[0].getAttribute("data-gdl")).toBe(
      "collapsable.header section-title"
    )
    expect(rows[1].getAttribute("data-gdl")).toBe("collapsable.header")
  })

  it("a collapsable header can hold a slot", () => {
    const { container } = render(() => (
      <Collapsable
        title="t"
        headerGdl="section-title"
        headerSlot={<i data-x />}
      >
        body
      </Collapsable>
    ))
    const row = container.querySelector('[data-gdl~="section-title"]')!
    expect(row.querySelector("[data-x]")).not.toBeNull()
    expect(row.classList.contains("relative")).toBe(true)
    expect(row.classList.contains("isolate")).toBe(true)
  })

  it("a collapsable root can hold a slot, and only then is positioned", () => {
    const { container } = render(() => (
      <>
        <Collapsable data-gdl="with" slot={<i data-y />}>
          body
        </Collapsable>
        <Collapsable data-gdl="without">body</Collapsable>
      </>
    ))
    const withSlot = container.querySelector('[data-gdl~="with"]')!
    expect(withSlot.lastElementChild?.matches("[data-y]")).toBe(true)
    expect(withSlot.classList.contains("relative")).toBe(true)
    expect(withSlot.classList.contains("isolate")).toBe(true)
    const without = container.querySelector('[data-gdl~="without"]')!
    expect(without.querySelector("[data-y]")).toBeNull()
    expect(without.classList.contains("relative")).toBe(false)
    expect(without.classList.contains("isolate")).toBe(false)
  })

  it("an empty slot adds no classes, a slot with content does", () => {
    const { container } = render(() => (
      <>
        <Collapsable
          data-gdl="empty"
          title="t"
          headerGdl="empty-row"
          slot={<Show when={false}>x</Show>}
          headerSlot={<Show when={false}>y</Show>}
        >
          body
        </Collapsable>
        <Collapsable
          data-gdl="full"
          title="t"
          headerGdl="full-row"
          slot={<Show when={true}>x</Show>}
          headerSlot={<Show when={true}>y</Show>}
        >
          body
        </Collapsable>
      </>
    ))
    const cls = (sel: string) =>
      [...container.querySelector(sel)!.classList].filter(
        (c) => c === "relative" || c === "isolate"
      )
    expect(cls('[data-gdl~="empty"]')).toEqual([])
    expect(cls('[data-gdl~="empty-row"]')).toEqual([])
    expect(cls('[data-gdl~="full"]')).toEqual(["relative", "isolate"])
    expect(cls('[data-gdl~="full-row"]')).toEqual(["relative", "isolate"])
  })
})
