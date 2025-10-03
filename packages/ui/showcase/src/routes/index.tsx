import { createFileRoute } from "@tanstack/solid-router"

export const Route = createFileRoute("/")({
  component: Index
})

function Index() {
  return (
    <div class="max-w-4xl">
      <div class="mb-8">
        <h1 class="text-4xl font-bold text-gray-900 mb-4">
          GDLauncher UI Components
        </h1>
        <p class="text-xl text-gray-600">
          A comprehensive showcase of all UI components used in GDLauncher,
          featuring interactive demos and configuration options.
        </p>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        <div class="bg-white p-6 rounded-lg border border-gray-200">
          <h3 class="text-lg font-semibold text-gray-900 mb-2">Components</h3>
          <p class="text-gray-600 mb-4">
            Explore 25+ interactive UI components with live examples and
            configurable properties.
          </p>
          <div class="text-3xl font-bold text-indigo-600">25+</div>
        </div>

        <div class="bg-white p-6 rounded-lg border border-gray-200">
          <h3 class="text-lg font-semibold text-gray-900 mb-2">Themes</h3>
          <p class="text-gray-600 mb-4">
            Switch between different themes to see how components adapt to
            various design systems.
          </p>
          <div class="text-3xl font-bold text-indigo-600">3</div>
        </div>

        <div class="bg-white p-6 rounded-lg border border-gray-200">
          <h3 class="text-lg font-semibold text-gray-900 mb-2">Interactive</h3>
          <p class="text-gray-600 mb-4">
            All components are fully interactive with real-time property updates
            and live code examples.
          </p>
          <div class="text-3xl font-bold text-indigo-600">100%</div>
        </div>
      </div>

      <div class="mt-12">
        <h2 class="text-2xl font-bold text-gray-900 mb-4">Getting Started</h2>
        <div class="bg-gray-50 p-6 rounded-lg">
          <ol class="list-decimal list-inside space-y-2 text-gray-700">
            <li>
              Browse the component list in the sidebar to explore available
              components
            </li>
            <li>
              Use the theme selector in the navbar to switch between different
              themes
            </li>
            <li>
              Each component page includes interactive demos and configuration
              options
            </li>
            <li>Copy code examples to use components in your own projects</li>
          </ol>
        </div>
      </div>
    </div>
  )
}
