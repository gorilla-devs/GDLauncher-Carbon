import { afterEach, expect, it, vi } from "vitest"
import { createSignal } from "solid-js"
import { render } from "solid-js/web"
import { Radio } from "./index"

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
  return {
    inputs: () => Array.from(root.querySelectorAll<HTMLInputElement>("input")),
    labels: () => Array.from(root.querySelectorAll<HTMLLabelElement>("label"))
  }
}

const options = [
  { value: "a", label: "A" },
  { value: "b", label: "B" }
]

// The restore runs in a microtask once the change handler has run.
const flush = () => Promise.resolve()

it("clicking the same option again calls onChange again when the parent did not apply the first choice", async () => {
  const calls = vi.fn()
  const { inputs, labels } = mount(() => (
    <Radio.group value="a" onChange={calls} options={options} />
  ))
  labels()[1].click()
  await flush()
  expect(inputs().map((i) => i.checked)).toEqual([true, false])
  labels()[1].click()
  await flush()
  inputs()[1].click()
  await flush()
  expect(calls.mock.calls).toEqual([["b"], ["b"], ["b"]])
  expect(inputs().map((i) => i.checked)).toEqual([true, false])
})

it("after a choice the parent did not apply, clicking the option drawn as selected does nothing", async () => {
  const calls = vi.fn()
  const { labels } = mount(() => (
    <Radio.group value="a" onChange={calls} options={options} />
  ))
  labels()[1].click()
  await flush()
  labels()[0].click()
  await flush()
  expect(calls.mock.calls).toEqual([["b"]])
})

it("a parent that applies the choice later ends with only that option checked", async () => {
  const calls = vi.fn()
  const [value, setValue] = createSignal("a")
  const { inputs, labels } = mount(() => (
    <Radio.group
      value={value()}
      onChange={(v) => {
        calls(v)
        setTimeout(() => setValue(v as string), 5)
      }}
      options={options}
    />
  ))
  labels()[1].click()
  await new Promise((resolve) => setTimeout(resolve, 20))
  expect(inputs().map((i) => i.checked)).toEqual([false, true])
  labels()[1].click()
  await flush()
  expect(calls.mock.calls).toEqual([["b"]])
  expect(inputs().map((i) => i.checked)).toEqual([false, true])
})

it("a refused choice in one group leaves another group's radios alone", async () => {
  const { inputs, labels } = mount(() => (
    <>
      <Radio.group name="g1" value="a" options={options} />
      <Radio.group name="g2" value="b" options={options} />
    </>
  ))
  labels()[1].click()
  await flush()
  expect(inputs().map((i) => i.checked)).toEqual([true, false, false, true])
})

it("a group whose name changed restores by its current name", async () => {
  const calls = vi.fn()
  const [name, setName] = createSignal("before")
  const { inputs, labels } = mount(() => (
    <Radio.group name={name()} value="a" onChange={calls} options={options} />
  ))
  setName("after")
  expect(inputs()[1].name).toBe("after")
  labels()[1].click()
  await flush()
  expect(inputs().map((i) => i.checked)).toEqual([true, false])
  labels()[1].click()
  await flush()
  expect(calls.mock.calls).toEqual([["b"], ["b"]])
})
