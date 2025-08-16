import { createFileRoute } from "@tanstack/solid-router"
import { Progress } from "../../../../src"
import ComponentDemo from "../../components/ComponentDemo"
import { createSignal, onMount } from "solid-js"

export const Route = createFileRoute("/components/progress")({
  component: ProgressPage
})

function ProgressPage() {
  const [progress1, setProgress1] = createSignal(0)
  const [progress2, _setProgress2] = createSignal(30)
  const [progress3, _setProgress3] = createSignal(75)

  onMount(() => {
    // Animate first progress bar
    let value = 0
    const interval = setInterval(() => {
      value += 1
      setProgress1(value)
      if (value >= 100) {
        value = 0
      }
    }, 100)

    return () => clearInterval(interval)
  })

  return (
    <div class="max-w-4xl">
      <div class="mb-8">
        <h1 class="text-4xl font-bold text-gray-900 mb-4">Progress</h1>
        <p class="text-xl text-gray-600">
          Unified progress indicator combining loading bars and progress bars
          with multiple sizes and styles.
        </p>
      </div>

      <ComponentDemo
        title="Animated Progress"
        description="Progress bar with animated progress"
        spacing="mb-20"
      >
        <div class="space-y-4">
          <Progress value={progress1()} max={100} />
          <div class="text-sm text-gray-600">
            Progress: {Math.round(progress1())}%
          </div>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Different Progress Values"
        description="Progress bars with various completion states"
        spacing="mb-20"
      >
        <div class="space-y-6">
          <div>
            <Progress value={progress2()} max={100} />
            <div class="text-sm text-gray-600 mt-2">30% Complete</div>
          </div>
          <div>
            <Progress value={progress3()} max={100} />
            <div class="text-sm text-gray-600 mt-2">75% Complete</div>
          </div>
          <div>
            <Progress value={100} max={100} />
            <div class="text-sm text-green-600 mt-2">100% Complete ✓</div>
          </div>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Different Sizes"
        description="Progress bars in various sizes"
        spacing="mb-20"
      >
        <div class="space-y-6">
          <div>
            <div class="text-sm text-gray-700 mb-3">Small</div>
            <Progress value={60} size="small" />
          </div>
          <div>
            <div class="text-sm text-gray-700 mb-3">Medium (Default)</div>
            <Progress value={60} size="medium" />
          </div>
          <div>
            <div class="text-sm text-gray-700 mb-3">Large</div>
            <Progress value={60} size="large" />
          </div>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Different Variants"
        description="Rounded vs square corner styles"
        spacing="mb-20"
      >
        <div class="space-y-6">
          <div>
            <div class="text-sm text-gray-700 mb-3">Rounded (Default)</div>
            <Progress value={75} variant="rounded" />
          </div>
          <div>
            <div class="text-sm text-gray-700 mb-3">Square</div>
            <Progress value={75} variant="square" />
          </div>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Custom Colors"
        description="Progress bars with different colors"
        spacing="mb-20"
      >
        <div class="space-y-6">
          <div>
            <div class="text-sm text-gray-700 mb-3">Primary (Default)</div>
            <Progress value={50} color="bg-primary-500" />
          </div>
          <div>
            <div class="text-sm text-gray-700 mb-3">Green</div>
            <Progress value={50} color="bg-green-500" />
          </div>
          <div>
            <div class="text-sm text-gray-700 mb-3">Red</div>
            <Progress value={50} color="bg-red-500" />
          </div>
          <div>
            <div class="text-sm text-gray-700 mb-3">Purple</div>
            <Progress value={50} color="bg-purple-500" />
          </div>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Indeterminate Loading"
        description="Progress bar for unknown duration tasks"
        spacing="mb-20"
      >
        <div class="space-y-4">
          <Progress indeterminate />
          <div class="text-sm text-gray-600">
            Indeterminate progress for ongoing operations
          </div>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Usage Examples"
        description="Real-world progress bar examples"
      >
        <div class="space-y-8">
          <div class="bg-gray-50 p-6 rounded-lg">
            <div class="text-sm font-medium text-gray-700 mb-3">
              File Upload
            </div>
            <Progress value={progress3()} size="small" color="bg-blue-500" />
            <div class="text-xs text-gray-500 mt-2">
              {progress3()}% uploaded
            </div>
          </div>

          <div class="bg-gray-50 p-6 rounded-lg">
            <div class="text-sm font-medium text-gray-700 mb-3">
              Download Progress
            </div>
            <Progress value={progress2()} color="bg-green-500" />
            <div class="text-xs text-gray-500 mt-2">
              Downloading... {progress2()}%
            </div>
          </div>

          <div class="bg-gray-50 p-6 rounded-lg">
            <div class="text-sm font-medium text-gray-700 mb-3">Processing</div>
            <Progress indeterminate size="large" />
            <div class="text-xs text-gray-500 mt-2">
              Processing your request...
            </div>
          </div>
        </div>
      </ComponentDemo>
    </div>
  )
}
