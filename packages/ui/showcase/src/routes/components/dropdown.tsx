import { createFileRoute } from "@tanstack/solid-router"
import { createSignal } from "solid-js"
import { Dropdown } from "../../../../src"
import ComponentDemo from "../../components/ComponentDemo"

export const Route = createFileRoute("/components/dropdown")({
  component: DropdownPage
})

function DropdownPage() {
  const basicOptions = [
    { label: "Option 1", key: "1" },
    { label: "Option 2", key: "2" },
    { label: "Option 3", key: "3" }
  ]

  const fruitOptions = [
    { label: "Apple", key: "apple" },
    { label: "Banana", key: "banana" },
    { label: "Orange", key: "orange" },
    { label: "Grape", key: "grape" },
    { label: "Strawberry", key: "strawberry" }
  ]

  const [selectedValue, setSelectedValue] = createSignal("apple")

  return (
    <div class="max-w-4xl">
      <div class="mb-8">
        <h1 class="text-4xl font-bold text-gray-900 mb-4">Dropdown</h1>
        <p class="text-xl text-gray-600">
          Interactive dropdown component with various configurations and
          options.
        </p>
      </div>

      <ComponentDemo
        title="Basic Dropdown"
        description="Simple dropdown with basic options"
      >
        <Dropdown options={basicOptions} />
      </ComponentDemo>

      <ComponentDemo
        title="Dropdown with Placeholder"
        description="Dropdown with custom placeholder text"
      >
        <Dropdown options={fruitOptions} placeholder="Select a fruit..." />
      </ComponentDemo>

      <ComponentDemo
        title="Controlled Dropdown"
        description="Dropdown with controlled value and change handler"
      >
        <div class="space-y-4">
          <Dropdown
            options={fruitOptions}
            value={selectedValue()}
            onChange={(option) => setSelectedValue(option.key.toString())}
          />
          <div class="text-gray-600">Selected: {selectedValue()}</div>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Rounded Dropdown"
        description="Dropdown with rounded corners"
      >
        <Dropdown options={basicOptions} rounded />
      </ComponentDemo>

      <ComponentDemo
        title="Disabled Dropdown"
        description="Non-interactive disabled state"
      >
        <Dropdown options={basicOptions} disabled />
      </ComponentDemo>

      <ComponentDemo
        title="Dropdown Button"
        description="Combination of button and dropdown for split actions"
      >
        <Dropdown.button
          options={fruitOptions}
          value={selectedValue()}
          onChange={(option) => setSelectedValue(option.key.toString())}
          onClick={() => alert("Button clicked!")}
        >
          Action
        </Dropdown.button>
      </ComponentDemo>
    </div>
  )
}
