import { createFileRoute } from "@tanstack/solid-router"
import { Radio } from "../../../../src"
import { createSignal, For } from "solid-js"
import ComponentDemo from "../../components/ComponentDemo"

export const Route = createFileRoute("/components/radio")({
  component: RadioPage
})

function RadioPage() {
  const [selectedSize, setSelectedSize] = createSignal("medium")
  const [selectedColor, setSelectedColor] = createSignal("blue")
  const [selectedPlan, setSelectedPlan] = createSignal("basic")

  return (
    <div class="max-w-4xl">
      <div class="mb-8">
        <h1 class="text-4xl font-bold text-gray-900 mb-4">Radio</h1>
        <p class="text-xl text-gray-600">
          Radio button component for single selection from a group of options.
        </p>
      </div>

      <ComponentDemo
        title="Basic Radio Group"
        description="Simple radio button group with controlled selection"
      >
        <div class="space-y-4">
          <div>
            <div class="text-sm font-medium text-gray-700 mb-2">
              Select Size:
            </div>
            <div class="space-y-2">
              <Radio
                value="small"
                checked={selectedSize() === "small"}
                onChange={() => setSelectedSize("small")}
              >
                Small
              </Radio>
              <Radio
                value="medium"
                checked={selectedSize() === "medium"}
                onChange={() => setSelectedSize("medium")}
              >
                Medium
              </Radio>
              <Radio
                value="large"
                checked={selectedSize() === "large"}
                onChange={() => setSelectedSize("large")}
              >
                Large
              </Radio>
            </div>
          </div>

          <div class="text-sm text-gray-600">
            Selected size: <strong>{selectedSize()}</strong>
          </div>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Radio States"
        description="Different radio button states including disabled"
      >
        <div class="space-y-2">
          <Radio value="unchecked" checked={false}>
            Normal
          </Radio>
          <Radio value="checked" checked={true}>
            Selected
          </Radio>
          <Radio value="disabled-unchecked" checked={false}>
            Disabled
          </Radio>
          <Radio value="disabled-checked" checked={true}>
            Selected & Disabled
          </Radio>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Color Selection"
        description="Radio group for color selection with visual indicators"
      >
        <div class="space-y-4">
          <div>
            <div class="text-sm font-medium text-gray-700 mb-3">
              Choose Color:
            </div>
            <div class="space-y-2">
              <For
                each={[
                  { value: "red", label: "Red", color: "bg-red-500" },
                  { value: "blue", label: "Blue", color: "bg-blue-500" },
                  { value: "green", label: "Green", color: "bg-green-500" },
                  { value: "purple", label: "Purple", color: "bg-purple-500" }
                ]}
              >
                {(color) => (
                  <div class="flex items-center space-x-3">
                    <Radio
                      value={color.value}
                      checked={selectedColor() === color.value}
                      onChange={() => setSelectedColor(color.value)}
                    >
                      <div class="flex items-center space-x-2">
                        <div
                          class={`w-4 h-4 rounded-full ${color.color}`}
                        ></div>
                        <span>{color.label}</span>
                      </div>
                    </Radio>
                  </div>
                )}
              </For>
            </div>
          </div>

          <div class="text-sm text-gray-600">
            Selected color: <strong>{selectedColor()}</strong>
          </div>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Pricing Plan Selection"
        description="Real-world example of radio buttons for plan selection"
      >
        <div class="space-y-4">
          <div class="text-sm font-medium text-gray-700 mb-3">Select Plan:</div>

          <div class="space-y-3">
            <For
              each={[
                {
                  value: "basic",
                  name: "Basic",
                  price: "$9/month",
                  features: ["1 GB Storage", "Email Support"]
                },
                {
                  value: "pro",
                  name: "Professional",
                  price: "$29/month",
                  features: ["10 GB Storage", "Priority Support", "Analytics"]
                },
                {
                  value: "enterprise",
                  name: "Enterprise",
                  price: "$99/month",
                  features: [
                    "100 GB Storage",
                    "24/7 Support",
                    "Advanced Analytics",
                    "Custom Integration"
                  ]
                }
              ]}
            >
              {(plan) => (
                <div
                  class={`border rounded-lg p-4 cursor-pointer transition-colors ${
                    selectedPlan() === plan.value
                      ? "border-indigo-500 bg-indigo-50"
                      : "border-gray-200 hover:border-gray-300"
                  }`}
                >
                  <Radio
                    value={plan.value}
                    checked={selectedPlan() === plan.value}
                    onChange={() => setSelectedPlan(plan.value)}
                  >
                    <div class="ml-3">
                      <div class="flex items-center justify-between">
                        <div class="font-medium text-gray-900">{plan.name}</div>
                        <div class="text-sm font-semibold text-gray-900">
                          {plan.price}
                        </div>
                      </div>
                      <div class="text-sm text-gray-500 mt-1">
                        {plan.features.join(" • ")}
                      </div>
                    </div>
                  </Radio>
                </div>
              )}
            </For>
          </div>

          <div class="text-sm text-gray-600">
            Selected plan: <strong>{selectedPlan()}</strong>
          </div>
        </div>
      </ComponentDemo>
    </div>
  )
}
