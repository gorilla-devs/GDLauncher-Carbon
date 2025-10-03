import { createFileRoute } from "@tanstack/solid-router"
import { Slider } from "../../../../src"
import { createSignal } from "solid-js"
import ComponentDemo from "../../components/ComponentDemo"

export const Route = createFileRoute("/components/slider")({
  component: SliderPage
})

function SliderPage() {
  const [value1, setValue1] = createSignal(50)
  const [value2, setValue2] = createSignal(25)
  const [value3, setValue3] = createSignal(75)

  return (
    <div class="max-w-4xl">
      <div class="mb-8">
        <h1 class="text-4xl font-bold text-gray-900 mb-4">Slider</h1>
        <p class="text-xl text-gray-600">
          Interactive slider component for selecting numeric values within a
          range.
        </p>
      </div>

      <ComponentDemo
        title="Basic Slider"
        description="Simple slider with controlled value"
      >
        <div class="space-y-4">
          <Slider value={value1()} onChange={setValue1} min={0} max={100} />
          <div class="text-sm text-gray-600">Current value: {value1()}</div>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Custom Range"
        description="Slider with custom min/max values"
      >
        <div class="space-y-4">
          <div>
            <label class="block text-sm font-medium text-gray-700 mb-2">
              Temperature: {value2()}°C
            </label>
            <Slider
              value={value2()}
              onChange={setValue2}
              min={-50}
              max={50}
              steps={5}
            />
          </div>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Volume Control Example"
        description="Practical example of a volume slider"
      >
        <div class="space-y-6">
          <div class="flex items-center space-x-4">
            <svg
              class="w-5 h-5 text-gray-600"
              fill="currentColor"
              viewBox="0 0 20 20"
            >
              <path
                fill-rule="evenodd"
                d="M9.383 3.076A1 1 0 0110 4v12a1 1 0 01-1.617.793L4.35 13.3a.5.5 0 00-.375-.175H2.5A1.5 1.5 0 011 11.625v-3.25A1.5 1.5 0 012.5 7h1.475a.5.5 0 00.375-.175l4.033-3.493zM14.5 7a.5.5 0 01.5.5v5a.5.5 0 01-1 0v-5a.5.5 0 01.5-.5z"
                clip-rule="evenodd"
              />
              <path d="M16.5 9a.5.5 0 01.5.5v1a.5.5 0 01-1 0v-1a.5.5 0 01.5-.5z" />
            </svg>
            <div class="flex-1">
              <Slider value={value3()} onChange={setValue3} min={0} max={100} />
            </div>
            <span class="text-sm font-medium text-gray-700 w-12 text-right">
              {value3()}%
            </span>
          </div>

          <div class="bg-gray-50 p-4 rounded-lg">
            <h4 class="text-sm font-medium text-gray-700 mb-2">
              Audio Settings
            </h4>
            <div class="space-y-3">
              <div class="flex items-center justify-between">
                <span class="text-sm text-gray-600">Master Volume</span>
                <span class="text-sm font-medium">{value3()}%</span>
              </div>
              <div class="flex items-center justify-between">
                <span class="text-sm text-gray-600">Status</span>
                <span
                  class={`text-sm font-medium ${value3() > 0 ? "text-green-600" : "text-red-600"}`}
                >
                  {value3() > 0 ? "Unmuted" : "Muted"}
                </span>
              </div>
            </div>
          </div>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Slider with Steps and Marks"
        description="Slider with stepped values and visual markers"
      >
        <div class="space-y-2">
          <label class="block text-sm font-medium text-gray-500">
            Volume Control (0-100 in steps of 10)
          </label>
          <Slider
            value={50}
            min={0}
            max={100}
            steps={10}
            marks={{
              0: "0%",
              25: "25%",
              50: "50%",
              75: "75%",
              100: "100%"
            }}
          />
        </div>
      </ComponentDemo>
    </div>
  )
}
