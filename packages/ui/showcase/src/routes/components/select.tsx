import { createFileRoute } from "@tanstack/solid-router"
import ComponentDemo from "../../components/ComponentDemo"

export const Route = createFileRoute("/components/select")({
  component: SelectPage
})

function SelectPage() {
  return (
    <div class="max-w-4xl">
      <div class="mb-8">
        <h1
          class="text-4xl font-bold mb-4"
          style={`color: rgb(var(--lightSlate-50))`}
        >
          Select
        </h1>
        <p class="text-xl" style={`color: rgb(var(--lightSlate-300))`}>
          Dropdown selection component - component showcase under development.
        </p>
      </div>

      <ComponentDemo
        title="Select Component"
        description="The Select component uses Kobalte primitives and requires proper configuration"
      >
        <div class="space-y-4">
          <div
            class="p-4 border rounded-md"
            style={`border-color: rgb(var(--darkSlate-600)); background-color: rgb(var(--darkSlate-800))`}
          >
            <p style={`color: rgb(var(--lightSlate-300))`}>
              The Select component is available but requires proper Kobalte
              integration for the showcase. Please refer to the component source
              code for implementation details.
            </p>
          </div>
        </div>
      </ComponentDemo>
    </div>
  )
}
