import { createFileRoute } from "@tanstack/solid-router"
import { NotificationsProvider, createNotification, Button } from "../../../../src"
import ComponentDemo from "../../components/ComponentDemo"

export const Route = createFileRoute("/components/notificationmanager")({
  component: NotificationManagerPage
})

function NotificationManagerPage() {
  return (
    <NotificationsProvider>
      <NotificationDemo />
    </NotificationsProvider>
  )
}

function NotificationDemo() {
  const addNotification = createNotification()

  const showSuccessNotification = () => {
    addNotification({
      name: "Success!",
      content: "Your action completed successfully.",
      type: "success",
      duration: 5000
    })
  }

  const showWarningNotification = () => {
    addNotification({
      name: "Warning",
      content: "Please review your settings before continuing.",
      type: "warning",
      duration: 7000
    })
  }

  const showErrorNotification = () => {
    addNotification({
      name: "Error occurred",
      content: "Something went wrong. Please try again later.",
      type: "error",
      duration: 10000
    })
  }

  const showLongContentNotification = () => {
    addNotification({
      name: "Long notification",
      content: "This is a notification with a very long content that demonstrates how the notification expands when there's more content to display. Click the arrow to expand or collapse the notification.",
      type: "success",
      duration: 8000
    })
  }

  const showCustomDurationNotification = () => {
    addNotification({
      name: "Custom duration",
      content: "This notification will disappear after 3 seconds.",
      type: "success",
      duration: 3000
    })
  }

  return (
    <div class="max-w-4xl">
      <div class="mb-8">
        <h1 class="text-4xl font-bold text-gray-900 mb-4">Notification Manager</h1>
        <p class="text-xl text-gray-600">
          Toast notification system with different types, expandable content, and progress indicators.
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
        title="Provider Setup"
        description="Wrap your app with NotificationsProvider to enable notifications"
      >
        <div class="bg-gray-50 p-4 rounded-lg">
          <p class="text-gray-700">
            Remember to add a div with id="notifications" to your HTML for the portal to render notifications.
            Notifications appear in the top-right corner and support hover-to-pause functionality.
          </p>
        </div>
      </ComponentDemo>

      {/* Add a notifications container for the portal */}
      <div id="notifications" class="fixed top-0 right-0 z-50" />
    </div>
  )
}