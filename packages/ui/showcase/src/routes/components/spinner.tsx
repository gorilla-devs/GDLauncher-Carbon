import { createFileRoute } from "@tanstack/solid-router"
import { Spinner } from "../../../../src"
import { createSignal } from "solid-js"
import ComponentDemo from "../../components/ComponentDemo"

export const Route = createFileRoute("/components/spinner")({
  component: SpinnerPage
})

function SpinnerPage() {
  const [loading, setLoading] = createSignal(false)

  const simulateLoading = () => {
    setLoading(true)
    setTimeout(() => setLoading(false), 3000)
  }

  return (
    <div class="max-w-4xl">
      <div class="mb-8">
        <h1 class="text-4xl font-bold text-gray-900 mb-4">Spinner</h1>
        <p class="text-xl text-gray-600">
          Loading spinner component to indicate ongoing processes.
        </p>
      </div>

      <ComponentDemo title="Basic Spinner" description="Simple loading spinner">
        <div class="flex items-center justify-center h-20">
          <Spinner />
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Spinner Sizes"
        description="Different spinner sizes"
      >
        <div class="flex items-center space-x-8">
          <div class="text-center">
            <Spinner class="h-4 w-4" />
            <div class="text-xs text-gray-500 mt-2">Small</div>
          </div>
          <div class="text-center">
            <Spinner class="h-6 w-6" />
            <div class="text-xs text-gray-500 mt-2">Medium</div>
          </div>
          <div class="text-center">
            <Spinner class="h-8 w-8" />
            <div class="text-xs text-gray-500 mt-2">Large</div>
          </div>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Loading Button Example"
        description="Spinner integrated with a button for loading states"
      >
        <div class="space-y-4">
          <button
            onClick={simulateLoading}
            disabled={loading()}
            class={`inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md text-white ${
              loading()
                ? "bg-gray-400 cursor-not-allowed"
                : "bg-indigo-600 hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500"
            }`}
          >
            {loading() ? (
              <>
                <Spinner class="h-4 w-4" />
                <span class="ml-2">Loading...</span>
              </>
            ) : (
              "Start Loading"
            )}
          </button>

          {loading() && (
            <div class="text-sm text-gray-600">
              Simulating a 3-second loading process...
            </div>
          )}
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Centered Loading State"
        description="Full-width centered loading indicator"
      >
        <div class="bg-gray-50 rounded-lg">
          <div class="flex flex-col items-center justify-center py-12">
            <Spinner class="h-8 w-8" />
            <p class="mt-4 text-gray-600">Loading content...</p>
            <p class="mt-2 text-sm text-gray-500">
              Please wait while we fetch your data
            </p>
          </div>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Inline Loading"
        description="Spinner used inline with text"
      >
        <div class="space-y-4">
          <div class="flex items-center space-x-2">
            <Spinner class="h-4 w-4" />
            <span class="text-gray-700">Processing your request...</span>
          </div>

          <div class="flex items-center space-x-2">
            <Spinner class="h-4 w-4" />
            <span class="text-gray-700">Saving changes...</span>
          </div>

          <div class="flex items-center space-x-2">
            <Spinner class="h-4 w-4" />
            <span class="text-gray-700">Uploading file...</span>
          </div>
        </div>
      </ComponentDemo>
    </div>
  )
}
