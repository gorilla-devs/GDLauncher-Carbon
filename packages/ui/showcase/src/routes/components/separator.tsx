import { createFileRoute } from "@tanstack/solid-router"
import { Separator } from "../../../../src"
import ComponentDemo from "../../components/ComponentDemo"

export const Route = createFileRoute("/components/separator")({
  component: SeparatorPage
})

function SeparatorPage() {
  return (
    <div class="max-w-4xl">
      <div class="mb-8">
        <h1
          class="text-4xl font-bold mb-4"
          style={`color: rgb(var(--lightSlate-50))`}
        >
          Separator
        </h1>
        <p class="text-xl" style={`color: rgb(var(--lightSlate-300))`}>
          Visual divider component to separate content sections.
        </p>
      </div>

      <ComponentDemo
        title="Horizontal Separator"
        description="Basic horizontal line separator"
      >
        <div class="space-y-4">
          <div style={`color: rgb(var(--lightSlate-100))`}>Content above</div>
          <Separator />
          <div style={`color: rgb(var(--lightSlate-100))`}>Content below</div>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Vertical Separator"
        description="Vertical separator for side-by-side content"
      >
        <div class="flex items-center h-12 space-x-4">
          <div style={`color: rgb(var(--lightSlate-100))`}>Left content</div>
          <Separator orientation="vertical" />
          <div style={`color: rgb(var(--lightSlate-100))`}>Right content</div>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Separator with Text"
        description="Separator with label or text in the middle"
      >
        <div class="space-y-4">
          <div style={`color: rgb(var(--lightSlate-100))`}>
            Sign in with email
          </div>
          <Separator>
            <span
              style={`color: rgb(var(--lightSlate-300)); background-color: rgb(var(--darkSlate-700)); padding: 0 1rem`}
            >
              OR
            </span>
          </Separator>
          <div style={`color: rgb(var(--lightSlate-100))`}>
            Sign in with Google
          </div>
        </div>
      </ComponentDemo>
    </div>
  )
}
