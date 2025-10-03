import { createFileRoute } from "@tanstack/solid-router"
import { TextArea } from "../../../../src"
import ComponentDemo from "../../components/ComponentDemo"
import { createSignal } from "solid-js"

export const Route = createFileRoute("/components/textarea")({
  component: TextAreaPage
})

function TextAreaPage() {
  const [value1, setValue1] = createSignal("")
  const [value2, setValue2] = createSignal(
    "This textarea has initial content that you can edit."
  )

  return (
    <div class="max-w-4xl">
      <div class="mb-8">
        <h1
          class="text-4xl font-bold mb-4"
          style={`color: rgb(var(--lightSlate-50))`}
        >
          TextArea
        </h1>
        <p class="text-xl" style={`color: rgb(var(--lightSlate-300))`}>
          Multi-line text input component for larger text content.
        </p>
      </div>

      <ComponentDemo
        title="Basic TextArea"
        description="Simple multi-line text input"
      >
        <div class="space-y-4">
          <TextArea
            placeholder="Enter your message here..."
            value={value1()}
            onInput={(e) => setValue1(e.target.value)}
          />
          <div style={`color: rgb(var(--lightSlate-300))`} class="text-sm">
            Character count: {value1().length}
          </div>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="TextArea with Initial Content"
        description="TextArea with predefined content"
      >
        <TextArea
          value={value2()}
          onInput={(e) => setValue2(e.target.value)}
          rows={4}
        />
      </ComponentDemo>

      <ComponentDemo
        title="Different Sizes"
        description="TextArea with different row heights"
      >
        <div class="space-y-4">
          <TextArea rows={2} placeholder="Small (2 rows)" />
          <TextArea rows={6} placeholder="Large (6 rows)" />
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Disabled TextArea"
        description="TextArea in disabled state"
      >
        <TextArea disabled={true} value="This textarea is disabled" />
      </ComponentDemo>
    </div>
  )
}
