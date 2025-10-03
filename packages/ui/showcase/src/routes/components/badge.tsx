import { createFileRoute } from "@tanstack/solid-router"
import { Badge } from "../../../../src"
import ComponentDemo from "../../components/ComponentDemo"

export const Route = createFileRoute("/components/badge")({
  component: BadgePage
})

function BadgePage() {
  return (
    <div class="max-w-4xl">
      <div class="mb-8">
        <h1 class="text-4xl font-bold text-gray-900 mb-4">Badge</h1>
        <p class="text-xl text-gray-600">
          Small badge component for labels, status indicators, and counts.
        </p>
      </div>

      <ComponentDemo
        title="Basic Badges"
        description="Simple badge with different content"
      >
        <div class="flex flex-wrap gap-2">
          <Badge>New</Badge>
          <Badge>Beta</Badge>
          <Badge>Pro</Badge>
          <Badge>Sale</Badge>
          <Badge>Popular</Badge>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Notification Badges"
        description="Badges used for notifications and counts"
      >
        <div class="flex items-center space-x-6">
          <div class="relative inline-block">
            <button class="px-4 py-2 bg-gray-100 rounded-md">Messages</button>
            <Badge>3</Badge>
          </div>
          <div class="relative inline-block">
            <button class="px-4 py-2 bg-gray-100 rounded-md">
              Notifications
            </button>
            <Badge>12</Badge>
          </div>
          <div class="relative inline-block">
            <button class="px-4 py-2 bg-gray-100 rounded-md">Cart</button>
            <Badge>99+</Badge>
          </div>
        </div>
      </ComponentDemo>
    </div>
  )
}
