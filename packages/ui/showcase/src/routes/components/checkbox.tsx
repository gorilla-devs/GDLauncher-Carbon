import { createFileRoute } from "@tanstack/solid-router"
import { Checkbox } from "../../../../src"
import { createSignal } from "solid-js"
import ComponentDemo from "../../components/ComponentDemo"

export const Route = createFileRoute("/components/checkbox")({
  component: CheckboxPage
})

function CheckboxPage() {
  const [checked1, setChecked1] = createSignal(false)
  const [checked2, setChecked2] = createSignal(true)
  const [checked3, setChecked3] = createSignal(false)

  return (
    <div class="max-w-4xl">
      <div class="mb-8">
        <h1 class="text-4xl font-bold text-gray-900 mb-4">Checkbox</h1>
        <p class="text-xl text-gray-600">
          Interactive checkbox component for boolean selections.
        </p>
      </div>

      <ComponentDemo
        title="Basic Checkbox"
        description="Simple checkbox with controlled state"
      >
        <div class="space-y-4">
          <Checkbox checked={checked1()} onChange={setChecked1}>
            Check me
          </Checkbox>
          <div class="text-sm text-gray-600">
            Status: {checked1() ? "Checked" : "Unchecked"}
          </div>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Checkbox States"
        description="Different checkbox states including disabled"
      >
        <div class="space-y-4">
          <Checkbox checked={checked2()} onChange={setChecked2}>
            Interactive checkbox
          </Checkbox>
          <Checkbox checked={true} disabled>
            Checked & Disabled
          </Checkbox>
          <Checkbox checked={false} disabled>
            Unchecked & Disabled
          </Checkbox>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Checkbox Group"
        description="Multiple checkboxes in a group"
      >
        <div>
          <div class="space-y-2">
            <Checkbox checked={checked1()} onChange={setChecked1}>
              Enable notifications
            </Checkbox>
            <Checkbox checked={checked2()} onChange={setChecked2}>
              Auto-save changes
            </Checkbox>
            <Checkbox checked={checked3()} onChange={setChecked3}>
              Dark mode
            </Checkbox>
          </div>

          <div class="mt-4 text-sm text-gray-600">
            Selected options:{" "}
            {[
              checked1() && "Notifications",
              checked2() && "Auto-save",
              checked3() && "Dark mode"
            ]
              .filter(Boolean)
              .join(", ") || "None"}
          </div>
        </div>
      </ComponentDemo>
    </div>
  )
}
