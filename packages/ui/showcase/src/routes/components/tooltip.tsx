import { createFileRoute } from "@tanstack/solid-router"
import { Tooltip, TooltipTrigger, TooltipContent } from "../../../../src"
import ComponentDemo from "../../components/ComponentDemo"

export const Route = createFileRoute("/components/tooltip")({
  component: TooltipPage
})

function TooltipPage() {
  return (
    <div class="max-w-4xl">
      <div class="mb-8">
        <h1
          class="text-4xl font-bold mb-4"
          style={`color: rgb(var(--lightSlate-50))`}
        >
          Tooltip
        </h1>
        <p class="text-xl" style={`color: rgb(var(--lightSlate-300))`}>
          Informational overlay that appears on hover or focus to provide
          additional context.
        </p>
      </div>

      <ComponentDemo
        title="Basic Tooltip"
        description="Simple tooltip that appears on hover"
      >
        <div class="flex items-center space-x-4">
          <Tooltip>
            <TooltipTrigger>
              <button
                class="px-4 py-2 rounded-md transition-colors"
                style={`background-color: rgb(var(--primary-500)); color: white`}
              >
                Hover me
              </button>
            </TooltipTrigger>
            <TooltipContent>This is a helpful tooltip</TooltipContent>
          </Tooltip>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Tooltip with Rich Content"
        description="Tooltips can contain rich content and formatting"
      >
        <div class="flex items-center justify-center space-x-6">
          <Tooltip>
            <TooltipTrigger>
              <button
                class="px-4 py-2 rounded-md transition-colors"
                style={`background-color: rgb(var(--darkSlate-600)); color: rgb(var(--lightSlate-50))`}
              >
                Rich Content
              </button>
            </TooltipTrigger>
            <TooltipContent>
              <div>
                <strong>Rich Tooltip</strong>
                <br />
                <em>With formatting and multiple lines</em>
              </div>
            </TooltipContent>
          </Tooltip>

          <Tooltip>
            <TooltipTrigger>
              <button
                class="px-4 py-2 rounded-md transition-colors"
                style={`background-color: rgb(var(--darkSlate-600)); color: rgb(var(--lightSlate-50))`}
              >
                Custom Styling
              </button>
            </TooltipTrigger>
            <TooltipContent class="bg-primary-500 text-white">
              Custom styled tooltip content
            </TooltipContent>
          </Tooltip>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Tooltip Examples"
        description="Various tooltip use cases"
      >
        <div class="grid grid-cols-2 gap-4">
          <Tooltip>
            <TooltipTrigger>
              <div class="p-4 border border-gray-300 rounded cursor-help">
                <div class="flex items-center space-x-2">
                  <span>Help Info</span>
                  <span class="text-gray-400">?</span>
                </div>
              </div>
            </TooltipTrigger>
            <TooltipContent>
              This provides helpful information about the feature
            </TooltipContent>
          </Tooltip>

          <Tooltip>
            <TooltipTrigger>
              <div class="p-4 bg-yellow-100 rounded cursor-pointer">
                <span class="text-yellow-800">Warning</span>
              </div>
            </TooltipTrigger>
            <TooltipContent class="bg-yellow-600 text-white">
              This action requires careful consideration
            </TooltipContent>
          </Tooltip>
        </div>
      </ComponentDemo>
    </div>
  )
}
