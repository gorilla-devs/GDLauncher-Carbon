import { afterEach, describe, expect, it } from "vitest"
import { render, screen, cleanup, fireEvent } from "solid-testing-library"
import { createSignal } from "solid-js"
import { Button } from "./Button"
import { Input } from "./Input"
import { Switch } from "./Switch"
import { Checkbox } from "./Checkbox"
import { Radio } from "./Radio"
import { Collapsable } from "./Collapsable"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue
} from "./Select"
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuGroup,
  ContextMenuGroupLabel,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger
} from "./Menu"
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger
} from "./DropdownMenu"
import { Toaster, toast } from "./Sonner"
import { withStates } from "./hooks"

// The hook tokens the shared components emit for their parts and states
// (the themes guide's "Controls" and "Toasts" tables).

afterEach(() => {
  cleanup()
  document.body.innerHTML = ""
})

const tokens = (el: Element | null) =>
  (el?.getAttribute("data-gdl") ?? "").split(" ").filter(Boolean)

const waitFor = (check: () => boolean, what: string) =>
  new Promise<void>((resolve, reject) => {
    const start = Date.now()
    const tick = () => {
      if (check()) return resolve()
      if (Date.now() - start > 3000) return reject(new Error(what))
      setTimeout(tick, 20)
    }
    tick()
  })

describe("withStates", () => {
  it("adds a token per state that is on, in order", () => {
    expect(withStates("tab", {})).toBe("tab")
    expect(withStates("tab", { active: false })).toBe("tab")
    expect(withStates("x", { a: true, b: 0, c: "yes" })).toBe("x x.a x.c")
  })
})

describe("buttons", () => {
  it("mark a loading button, and drop the mark when it's done", () => {
    const [loading, setLoading] = createSignal(true)
    render(() => (
      <Button data-testid="b" loading={loading()}>
        go
      </Button>
    ))
    const b = screen.getByTestId("b")
    expect(tokens(b)).toContain("button.loading")
    setLoading(false)
    expect(tokens(b)).not.toContain("button.loading")
  })

  it("a transparent button has no inline background, so a hook rule can set one", () => {
    render(() => (
      <Button type="transparent" data-testid="t" disabled>
        x
      </Button>
    ))
    const t = screen.getByTestId("t")
    expect(t.style.background).toBe("")
    expect(t.style.backgroundColor).toBe("")
    expect(t.classList.contains("bg-black/90")).toBe(true)
    // Disabled keeps the solid fill (the inline style used to hide stripes).
    expect(t.classList.contains("disabled-stripes")).toBe(false)
  })
})

describe("inputs", () => {
  it("mark their parts and states", () => {
    const [error, setError] = createSignal<string | undefined>(undefined)
    const { container } = render(() => (
      <Input disabled icon={<i />} errorMessage={error()} />
    ))
    const input = container.querySelector('[data-gdl~="input"]')!
    expect(tokens(input)).toEqual(["input", "input.disabled"])
    expect(container.querySelector('[data-gdl="input.icon"]')).not.toBeNull()
    expect(container.querySelector('[data-gdl="input.error"]')).toBeNull()
    setError("bad")
    expect(tokens(input)).toEqual(["input", "input.disabled", "input.invalid"])
  })

  it("mark the error message", () => {
    const { container } = render(() => <Input errorMessage="bad" />)
    expect(
      container.querySelector('[data-gdl="input.error"]')!.textContent
    ).toBe("bad")
  })
})

describe("switches, checkboxes and radios", () => {
  it("a switch marks its parts and follows its state", () => {
    const [on, setOn] = createSignal(false)
    const { container } = render(() => <Switch checked={on()} />)
    const sw = container.querySelector('[data-gdl~="switch"]')!
    expect(tokens(sw)).toEqual(["switch"])
    expect(sw.querySelector('[data-gdl="switch.track"]')).not.toBeNull()
    expect(sw.querySelector('[data-gdl="switch.thumb"]')).not.toBeNull()
    setOn(true)
    expect(tokens(sw)).toEqual(["switch", "switch.checked"])
  })

  it("a disabled switch says so", () => {
    const { container } = render(() => <Switch disabled />)
    expect(tokens(container.querySelector('[data-gdl~="switch"]'))).toEqual([
      "switch",
      "switch.disabled"
    ])
  })

  it("a checkbox follows a click and its prop", () => {
    const [checked, setChecked] = createSignal(false)
    const { container } = render(() => <Checkbox checked={checked()} />)
    const box = container.querySelector('[data-gdl~="checkbox"]')!
    expect(tokens(box)).toEqual(["checkbox"])
    expect(box.querySelector('[data-gdl="checkbox.box"]')).not.toBeNull()
    expect(box.querySelector('[data-gdl="checkbox.mark"]')).not.toBeNull()
    setChecked(true)
    expect(tokens(box)).toEqual(["checkbox", "checkbox.checked"])
  })

  it("a switch that loads or is indeterminate isn't checked", () => {
    const [loading, setLoading] = createSignal(true)
    const [mixed, setMixed] = createSignal(false)
    const { container } = render(() => (
      <Switch checked isLoading={loading()} isIndeterminate={mixed()} />
    ))
    const sw = () => tokens(container.querySelector('[data-gdl~="switch"]'))
    expect(sw()).toEqual(["switch", "switch.loading"])
    setLoading(false)
    setMixed(true)
    expect(sw()).toEqual(["switch", "switch.indeterminate"])
    setMixed(false)
    expect(sw()).toEqual(["switch", "switch.checked"])
  })

  it("a checkbox showing a dash is indeterminate until it's ticked", () => {
    const [checked, setChecked] = createSignal(false)
    const { container } = render(() => (
      <Checkbox indeterminate checked={checked()} />
    ))
    const box = () => tokens(container.querySelector('[data-gdl~="checkbox"]'))
    expect(box()).toEqual(["checkbox", "checkbox.indeterminate"])
    setChecked(true)
    expect(box()).toEqual(["checkbox", "checkbox.checked"])
  })

  it("a disabled checkbox says so", () => {
    const { container } = render(() => <Checkbox disabled />)
    expect(tokens(container.querySelector('[data-gdl~="checkbox"]'))).toEqual([
      "checkbox",
      "checkbox.disabled"
    ])
  })

  it("radios mark the chosen one", () => {
    const [value, setValue] = createSignal("a")
    const { container } = render(() => (
      <Radio.group
        value={value()}
        options={[
          { value: "a", label: "A" },
          { value: "b", label: "B" }
        ]}
      />
    ))
    const radios = () =>
      [...container.querySelectorAll('[data-gdl~="radio"]')].map(tokens)
    expect(radios()).toEqual([["radio", "radio.checked"], ["radio"]])
    expect(container.querySelectorAll('[data-gdl="radio.dot"]')).toHaveLength(2)
    setValue("b")
    expect(radios()).toEqual([["radio"], ["radio", "radio.checked"]])
  })
})

describe("collapsables", () => {
  it("mark their parts, and whether they're open", () => {
    const { container } = render(() => (
      <Collapsable
        data-gdl="library.group"
        headerGdl="section-title"
        title="Group"
        count={2}
        onClear={() => {}}
      >
        body
      </Collapsable>
    ))
    const root = container.querySelector('[data-gdl~="collapsable"]')!
    expect(tokens(root)).toEqual([
      "collapsable",
      "collapsable.open",
      "library.group"
    ])
    const header = root.querySelector('[data-gdl~="collapsable.header"]')!
    expect(tokens(header)).toEqual(["collapsable.header", "section-title"])
    for (const part of ["chevron", "title", "count", "clear", "body"]) {
      expect(
        root.querySelector(`[data-gdl="collapsable.${part}"]`),
        part
      ).not.toBeNull()
    }
    fireEvent.click(header)
    expect(tokens(root)).toEqual(["collapsable", "library.group"])
  })
})

describe("menus", () => {
  it("a right-click menu marks its items, labels, lines and disabled items", () => {
    render(() => (
      <ContextMenu>
        <ContextMenuTrigger data-testid="area">area</ContextMenuTrigger>
        <ContextMenuContent>
          <ContextMenuGroup>
            <ContextMenuGroupLabel>Group</ContextMenuGroupLabel>
            <ContextMenuSeparator />
            <ContextMenuItem>One</ContextMenuItem>
            <ContextMenuItem disabled>Two</ContextMenuItem>
          </ContextMenuGroup>
        </ContextMenuContent>
      </ContextMenu>
    ))
    fireEvent.contextMenu(screen.getByTestId("area"))
    expect(tokens(screen.getByText("One"))).toEqual(["context-menu.item"])
    expect(tokens(screen.getByText("Two"))).toEqual([
      "context-menu.item",
      "context-menu.item.disabled"
    ])
    expect(tokens(screen.getByText("Group"))).toEqual(["context-menu.label"])
    expect(
      document.querySelector('[data-gdl="context-menu.separator"]')
    ).not.toBeNull()
  })

  it("a drop-down marks its items, and the ticked and chosen ones", () => {
    const [size, setSize] = createSignal("1")
    const [ticked, setTicked] = createSignal(false)
    render(() => (
      <DropdownMenu open>
        <DropdownMenuTrigger>open</DropdownMenuTrigger>
        <DropdownMenuContent>
          <DropdownMenuLabel>Label</DropdownMenuLabel>
          <DropdownMenuSeparator />
          <DropdownMenuItem disabled>Off</DropdownMenuItem>
          <DropdownMenuCheckboxItem checked={ticked()}>
            Tick
          </DropdownMenuCheckboxItem>
          <DropdownMenuRadioGroup value={size()}>
            <DropdownMenuRadioItem value="1">Small</DropdownMenuRadioItem>
            <DropdownMenuRadioItem value="2">Big</DropdownMenuRadioItem>
          </DropdownMenuRadioGroup>
        </DropdownMenuContent>
      </DropdownMenu>
    ))
    const item = (text: string) =>
      tokens(screen.getByText(text).closest('[data-gdl~="dropdown.item"]'))
    expect(tokens(screen.getByText("Label"))).toEqual(["dropdown.label"])
    expect(
      document.querySelector('[data-gdl="dropdown.separator"]')
    ).not.toBeNull()
    expect(item("Off")).toEqual(["dropdown.item", "dropdown.item.disabled"])
    expect(item("Tick")).toEqual(["dropdown.item"])
    expect(item("Small")).toEqual(["dropdown.item", "dropdown.item.checked"])
    expect(item("Big")).toEqual(["dropdown.item"])
    setTicked(true)
    setSize("2")
    expect(item("Tick")).toEqual(["dropdown.item", "dropdown.item.checked"])
    expect(item("Small")).toEqual(["dropdown.item"])
    expect(item("Big")).toEqual(["dropdown.item", "dropdown.item.checked"])
  })
})

describe("a caller's hooks", () => {
  it("are added to the items' own, not put in their place", async () => {
    render(() => (
      <>
        <DropdownMenu open>
          <DropdownMenuTrigger>open</DropdownMenuTrigger>
          <DropdownMenuContent>
            <DropdownMenuLabel data-gdl="my.label">Label</DropdownMenuLabel>
            <DropdownMenuSeparator data-gdl="my.line" />
            <DropdownMenuItem data-gdl="my.item">Item</DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
        <ContextMenu>
          <ContextMenuTrigger data-testid="area">area</ContextMenuTrigger>
          <ContextMenuContent>
            <ContextMenuItem data-gdl="my.entry">Entry</ContextMenuItem>
          </ContextMenuContent>
        </ContextMenu>
        <Select
          open
          options={["a"]}
          value="a"
          itemComponent={(p) => (
            <SelectItem item={p.item} data-gdl="my.choice">
              {p.item.rawValue}
            </SelectItem>
          )}
        >
          <SelectTrigger>
            <SelectValue<string> data-gdl="my.value">
              {(s) => s.selectedOption()}
            </SelectValue>
          </SelectTrigger>
          <SelectContent />
        </Select>
      </>
    ))
    fireEvent.contextMenu(screen.getByTestId("area"))
    const at = (sel: string) => tokens(document.querySelector(sel))
    expect(at('[data-gdl~="my.label"]')).toEqual(["dropdown.label", "my.label"])
    expect(at('[data-gdl~="my.line"]')).toEqual([
      "dropdown.separator",
      "my.line"
    ])
    expect(at('[data-gdl~="my.item"]')).toEqual(["dropdown.item", "my.item"])
    expect(at('[data-gdl~="my.entry"]')).toEqual([
      "context-menu.item",
      "my.entry"
    ])
    expect(at('[data-gdl~="my.value"]')).toEqual(["select.value", "my.value"])
    await waitFor(
      () => !!document.querySelector('[data-gdl~="my.choice"]'),
      "no select item"
    )
    expect(at('[data-gdl~="my.choice"]')).toEqual([
      "select.item",
      "select.item.selected",
      "my.choice"
    ])
  })
})

describe("selects", () => {
  it("the box has no important background, so a hook rule can set one", () => {
    render(() => (
      <Select options={["a"]} itemComponent={() => null}>
        <SelectTrigger data-testid="trigger">
          <SelectValue<string>>{(s) => s.selectedOption()}</SelectValue>
        </SelectTrigger>
      </Select>
    ))
    const trigger = screen.getByTestId("trigger")
    expect(tokens(trigger)).toContain("select")
    expect([...trigger.classList].filter((c) => c.startsWith("!"))).toEqual([])
    expect(trigger.classList.contains("gdl-select-fill")).toBe(true)
    expect(trigger.querySelector('[data-gdl="select.value"]')).not.toBeNull()
    expect(trigger.querySelector('[data-gdl="select.icon"]')).not.toBeNull()
  })

  it("its list marks the chosen and the disabled choices", async () => {
    const [value, setValue] = createSignal("b")
    render(() => (
      <Select
        open
        options={["a", "b", "c"]}
        value={value()}
        optionDisabled={(o) => o === "c"}
        itemComponent={(p) => (
          <SelectItem item={p.item}>{p.item.rawValue}</SelectItem>
        )}
      >
        <SelectTrigger>
          <SelectValue<string>>{(s) => s.selectedOption()}</SelectValue>
        </SelectTrigger>
        <SelectContent />
      </Select>
    ))
    // The chosen item also holds the tick's "Checked" title.
    const item = (text: string) =>
      tokens(
        [...document.querySelectorAll('[data-gdl~="select.item"]')].find((el) =>
          el.textContent?.trim().endsWith(text)
        ) ?? null
      )
    await waitFor(
      () => document.querySelectorAll('[data-gdl~="select.item"]').length === 3,
      "no select items"
    )
    expect(item("a")).toEqual(["select.item"])
    expect(item("b")).toEqual(["select.item", "select.item.selected"])
    expect(item("c")).toEqual(["select.item", "select.item.disabled"])
    setValue("a")
    expect(item("a")).toEqual(["select.item", "select.item.selected"])
    expect(item("b")).toEqual(["select.item"])
  })
})

describe("toasts", () => {
  it("mark their kind and parts", async () => {
    render(() => <Toaster />)
    toast.error("Broke", {
      description: "More",
      action: { label: "Retry", onClick: () => {} },
      cancel: { label: "Skip", onClick: () => {} }
    })
    await screen.findByText("Broke")
    const el = () => document.querySelector("[data-somoto-toast]")
    await waitFor(
      () => tokens(el()).join(" ") === "toast toast.error",
      `toast hooks: ${el()?.getAttribute("data-gdl")}`
    )
    for (const part of [
      "title",
      "description",
      "icon",
      "action",
      "cancel",
      "close"
    ]) {
      await waitFor(
        () => !!el()?.querySelector(`[data-gdl="toast.${part}"]`),
        `no toast.${part}`
      )
    }
  })

  it("a toast that changes kind changes its hook", async () => {
    render(() => <Toaster />)
    const id = toast.loading("Working")
    await screen.findByText("Working")
    const el = () => document.querySelector("[data-somoto-toast]")
    await waitFor(
      () => tokens(el()).includes("toast.loading"),
      `loading: ${el()?.getAttribute("data-gdl")}`
    )
    toast.success("Done", { id })
    await waitFor(
      () => tokens(el()).join(" ") === "toast toast.success",
      `success: ${el()?.getAttribute("data-gdl")}`
    )
  })
})
