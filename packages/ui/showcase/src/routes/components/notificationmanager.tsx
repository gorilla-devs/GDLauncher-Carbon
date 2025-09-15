import { createFileRoute } from "@tanstack/solid-router"
import { toast, Button } from "../../../../src"
import ComponentDemo from "../../components/ComponentDemo"

export const Route = createFileRoute("/components/notificationmanager")({
  component: NotificationManagerPage
})

function NotificationManagerPage() {
  return <NotificationDemo />
}

function NotificationDemo() {
  const showSuccessNotification = () => {
    toast.success("Success!", {
      description: "Your action completed successfully.",
      duration: 5000
    })
  }

  const showWarningNotification = () => {
    toast.warning("Warning", {
      description: "Please review your settings before continuing.",
      duration: 7000
    })
  }

  const showErrorNotification = () => {
    toast.error("Error occurred", {
      description: "Something went wrong. Please try again later.",
      duration: 10000
    })
  }

  const showLongContentNotification = () => {
    toast.success("Long notification", {
      description:
        "This is a notification with a very long content that demonstrates how the notification expands when there's more content to display. Click the arrow to expand or collapse the notification.",
      duration: 8000
    })
  }

  const showCustomDurationNotification = () => {
    toast.success("Custom duration", {
      description: "This notification will disappear after 3 seconds.",
      duration: 3000
    })
  }

  return (
    <div class="max-w-4xl">
      <div class="mb-8">
        <h1
          class="text-4xl font-bold mb-4"
          style={`color: rgb(var(--lightSlate-50))`}
        >
          Sonner Toasts
        </h1>
        <p class="text-xl" style={`color: rgb(var(--lightSlate-300))`}>
          Toast notification system powered by Sonner with different types and
          custom durations.
        </p>
      </div>

      <ComponentDemo
        title="Notification Types"
        description="Different notification types for various use cases"
      >
        <div class="flex flex-wrap gap-4">
          <Button type="primary" onClick={showSuccessNotification}>
            Show Success
          </Button>
          <Button type="secondary" onClick={showWarningNotification}>
            Show Warning
          </Button>
          <Button type="outline" onClick={showErrorNotification}>
            Show Error
          </Button>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Custom Duration"
        description="Control how long notifications stay visible"
      >
        <Button type="primary" onClick={showCustomDurationNotification}>
          Show 3s Notification
        </Button>
      </ComponentDemo>

      <ComponentDemo
        title="Expandable Content"
        description="Notifications with long content can be expanded for better readability"
      >
        <Button type="primary" onClick={showLongContentNotification}>
          Show Long Content
        </Button>
      </ComponentDemo>

      <ComponentDemo
        title="Setup"
        description="Add Toaster component to your app root to enable toast notifications"
      >
        <div
          class="p-4 rounded-lg"
          style="background-color: rgb(var(--darkSlate-700)); border: 1px solid rgb(var(--darkSlate-600))"
        >
          <p style="color: rgb(var(--lightSlate-300))">
            Simply add {"<Toaster />"} to your app root. Toasts appear in the
            bottom-left corner and automatically handle positioning and
            stacking.
          </p>
        </div>
      </ComponentDemo>
    </div>
  )
}
