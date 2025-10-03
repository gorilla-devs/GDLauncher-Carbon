import { createFileRoute } from "@tanstack/solid-router"
import { Tabs, TabList, TabPanel, Tab } from "../../../../src"
import ComponentDemo from "../../components/ComponentDemo"

export const Route = createFileRoute("/components/tabs")({
  component: TabsPage
})

function TabsPage() {
  return (
    <div class="max-w-4xl">
      <div class="mb-8">
        <h1 class="text-4xl font-bold text-gray-900 mb-4">Tabs</h1>
        <p class="text-xl text-gray-600">
          Tab navigation component for organizing content into separate views.
        </p>
      </div>

      <ComponentDemo
        title="Basic Tabs"
        description="Simple tab interface with multiple panels"
      >
        <Tabs>
          <TabList>
            <Tab>Overview</Tab>
            <Tab>Analytics</Tab>
            <Tab>Settings</Tab>
          </TabList>
          <TabPanel>
            <div class="p-4">
              <h3 class="font-semibold text-gray-900 mb-2">Overview</h3>
              <p class="text-gray-600">
                Welcome to your dashboard overview. Here you can see a summary
                of your account activity.
              </p>
            </div>
          </TabPanel>
          <TabPanel>
            <div class="p-4">
              <h3 class="font-semibold text-gray-900 mb-2">Analytics</h3>
              <p class="text-gray-600">
                View detailed analytics and performance metrics for your
                account.
              </p>
            </div>
          </TabPanel>
          <TabPanel>
            <div class="p-4">
              <h3 class="font-semibold text-gray-900 mb-2">Settings</h3>
              <p class="text-gray-600">
                Configure your account settings and preferences.
              </p>
            </div>
          </TabPanel>
        </Tabs>
      </ComponentDemo>

      <ComponentDemo
        title="Content-Rich Tabs"
        description="Tabs with more complex content in each panel"
      >
        <Tabs>
          <TabList>
            <Tab>Profile</Tab>
            <Tab>Security</Tab>
            <Tab>Notifications</Tab>
          </TabList>
          <TabPanel>
            <div class="p-4 space-y-4">
              <div>
                <label class="block text-sm font-medium text-gray-700">
                  Full Name
                </label>
                <input
                  type="text"
                  class="mt-1 block w-full border border-gray-300 rounded-md px-3 py-2"
                  placeholder="John Doe"
                />
              </div>
              <div>
                <label class="block text-sm font-medium text-gray-700">
                  Email
                </label>
                <input
                  type="email"
                  class="mt-1 block w-full border border-gray-300 rounded-md px-3 py-2"
                  placeholder="john@example.com"
                />
              </div>
            </div>
          </TabPanel>
          <TabPanel>
            <div class="p-4 space-y-4">
              <div class="flex items-center justify-between">
                <div>
                  <div class="font-medium">Two-Factor Authentication</div>
                  <div class="text-sm text-gray-500">
                    Add an extra layer of security
                  </div>
                </div>
                <button class="px-4 py-2 bg-indigo-600 text-white rounded-md text-sm">
                  Enable
                </button>
              </div>
              <div class="flex items-center justify-between">
                <div>
                  <div class="font-medium">Password</div>
                  <div class="text-sm text-gray-500">
                    Last updated 30 days ago
                  </div>
                </div>
                <button class="px-4 py-2 border border-gray-300 rounded-md text-sm">
                  Change
                </button>
              </div>
            </div>
          </TabPanel>
          <TabPanel>
            <div class="p-4 space-y-4">
              <div class="flex items-center justify-between">
                <div>
                  <div class="font-medium">Email Notifications</div>
                  <div class="text-sm text-gray-500">
                    Receive updates via email
                  </div>
                </div>
                <input type="checkbox" checked class="rounded" />
              </div>
              <div class="flex items-center justify-between">
                <div>
                  <div class="font-medium">Push Notifications</div>
                  <div class="text-sm text-gray-500">
                    Receive push notifications
                  </div>
                </div>
                <input type="checkbox" class="rounded" />
              </div>
            </div>
          </TabPanel>
        </Tabs>
      </ComponentDemo>
    </div>
  )
}
