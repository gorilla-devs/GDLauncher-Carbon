import { createFileRoute } from "@tanstack/solid-router"
import { Steps } from "../../../../src"
import ComponentDemo from "../../components/ComponentDemo"
import { createSignal } from "solid-js"

export const Route = createFileRoute("/components/steps")({
  component: StepsPage
})

function StepsPage() {
  const [currentStep, setCurrentStep] = createSignal(1)

  const basicSteps = ["Account", "Profile", "Settings", "Complete"]

  const customSteps = [
    { label: "Create Account", icon: "1" },
    { label: "Setup Profile", icon: "2" },
    { label: "Configure Settings", icon: "3" },
    { label: "Complete Setup", icon: "4" }
  ]

  return (
    <div class="max-w-4xl">
      <div class="mb-8">
        <h1
          class="text-4xl font-bold mb-4"
          style={`color: rgb(var(--lightSlate-50))`}
        >
          Steps
        </h1>
        <p class="text-xl" style={`color: rgb(var(--lightSlate-300))`}>
          Step indicator component for multi-step processes and workflows.
        </p>
      </div>

      <ComponentDemo
        title="Basic Steps"
        description="Simple step indicator with progress using string array"
      >
        <div class="space-y-6">
          <Steps steps={basicSteps} currentStep={currentStep()} />
          <div class="flex space-x-2">
            <button
              onClick={() => setCurrentStep(Math.max(0, currentStep() - 1))}
              disabled={currentStep() === 0}
              class="px-4 py-2 rounded-md transition-colors disabled:opacity-50"
              style={`background-color: rgb(var(--darkSlate-600)); color: rgb(var(--lightSlate-50))`}
            >
              Previous
            </button>
            <button
              onClick={() =>
                setCurrentStep(
                  Math.min(basicSteps.length - 1, currentStep() + 1)
                )
              }
              disabled={currentStep() === basicSteps.length - 1}
              class="px-4 py-2 rounded-md transition-colors disabled:opacity-50"
              style={`background-color: rgb(var(--primary-500)); color: white`}
            >
              Next
            </button>
          </div>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Custom Steps with Labels"
        description="Steps with custom objects containing label and icon"
      >
        <Steps steps={customSteps} currentStep={2} />
      </ComponentDemo>

      <ComponentDemo
        title="Steps with Custom Icons"
        description="Steps can include custom icons and labels"
      >
        <Steps
          steps={[
            { label: "Start Process", icon: "🚀" },
            { label: "In Progress", icon: "⚙️" },
            { label: "Complete", icon: "✅" }
          ]}
          currentStep={1}
        />
      </ComponentDemo>
    </div>
  )
}
