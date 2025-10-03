import { createFileRoute } from "@tanstack/solid-router"
import { Button } from "../../../../src"
import ComponentDemo from "../../components/ComponentDemo"

export const Route = createFileRoute("/components/button")({
  component: ButtonPage
})

function ButtonPage() {
  return (
    <div class="max-w-4xl">
      <div class="mb-8">
        <h1 class="text-4xl font-bold text-gray-900 mb-4">Button</h1>
        <p class="text-xl text-gray-600">
          Interactive button component with multiple variants and sizes.
        </p>
      </div>

      <ComponentDemo
        title="Button Types"
        description="Different button types for various use cases"
      >
        <div class="flex flex-wrap gap-4">
          <Button type="primary">Primary</Button>
          <Button type="secondary">Secondary</Button>
          <Button type="outline">Outline</Button>
          <Button type="glow">Glow</Button>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Button Sizes"
        description="Different sizes to fit various contexts"
      >
        <div class="flex flex-wrap items-center gap-4">
          <Button size="large">Large</Button>
          <Button size="medium">Medium</Button>
          <Button size="small">Small</Button>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Button States"
        description="Various button states including disabled"
      >
        <div class="flex flex-wrap gap-4">
          <Button>Default</Button>
          <Button disabled>Disabled</Button>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Interactive Example"
        description="A button with click handler"
      >
        <Button type="primary" onClick={() => alert("Button clicked!")}>
          Click me!
        </Button>
      </ComponentDemo>
    </div>
  )
}
