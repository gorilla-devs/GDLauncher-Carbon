import { createFileRoute } from "@tanstack/solid-router"
import { Input } from "../../../../src"
import { createSignal } from "solid-js"
import ComponentDemo from "../../components/ComponentDemo"

export const Route = createFileRoute("/components/input")({
  component: InputPage
})

function InputPage() {
  const [value, setValue] = createSignal("")
  const [emailValue, setEmailValue] = createSignal("")
  const [passwordValue, setPasswordValue] = createSignal("")

  return (
    <div class="max-w-4xl">
      <div class="mb-8">
        <h1 class="text-4xl font-bold text-gray-900 mb-4">Input</h1>
        <p class="text-xl text-gray-600">
          Text input component with various configurations and types.
        </p>
      </div>

      <ComponentDemo
        title="Basic Input"
        description="Simple text input with placeholder"
      >
        <Input
          placeholder="Enter text here..."
          value={value()}
          onInput={(e) => setValue(e.target.value)}
        />
        <div class="mt-2 text-sm text-gray-600">Current value: {value()}</div>
      </ComponentDemo>

      <ComponentDemo
        title="Input Types"
        description="Different input types for various use cases"
      >
        <div class="space-y-4">
          <Input
            type="email"
            placeholder="Email address"
            value={emailValue()}
            onInput={(e) => setEmailValue(e.target.value)}
          />
          <Input
            type="password"
            placeholder="Password"
            value={passwordValue()}
            onInput={(e) => setPasswordValue(e.target.value)}
          />
          <Input type="number" placeholder="Number" />
          <Input type="search" placeholder="Search..." />
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Input States"
        description="Different states including disabled and readonly"
      >
        <div class="space-y-4">
          <Input placeholder="Normal input" />
          <Input placeholder="Disabled input" disabled />
          <Input placeholder="Readonly input" readonly value="Cannot edit" />
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Input with Labels"
        description="Inputs with associated labels"
      >
        <div class="space-y-4">
          <div class="space-y-2">
            <label class="block text-sm font-medium text-gray-700">
              Full Name
            </label>
            <Input placeholder="John Doe" />
          </div>

          <div class="space-y-2">
            <label class="block text-sm font-medium text-gray-700">
              Email Address
            </label>
            <Input type="email" placeholder="john@example.com" />
          </div>
        </div>
      </ComponentDemo>
    </div>
  )
}
