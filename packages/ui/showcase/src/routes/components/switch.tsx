import { createFileRoute } from "@tanstack/solid-router"
import { Switch } from "../../../../src"
import { createSignal } from "solid-js"
import ComponentDemo from "../../components/ComponentDemo"

export const Route = createFileRoute("/components/switch")({
  component: SwitchPage
})

function SwitchPage() {
  const [enabled1, setEnabled1] = createSignal(false)
  const [enabled2, setEnabled2] = createSignal(true)
  const [enabled3, setEnabled3] = createSignal(false)

  return (
    <div class="max-w-4xl">
      <div class="mb-8">
        <h1 class="text-4xl font-bold text-gray-900 mb-4">Switch</h1>
        <p class="text-xl text-gray-600">
          Toggle switch component for boolean settings.
        </p>
      </div>

      <ComponentDemo
        title="Basic Switch"
        description="Simple toggle switch with controlled state"
      >
        <div class="space-y-4">
          <div class="flex items-center space-x-3">
            <Switch checked={enabled1()} onChange={setEnabled1} />
            <span class="text-sm text-gray-700">
              Notifications {enabled1() ? "enabled" : "disabled"}
            </span>
          </div>
          <div class="text-sm text-gray-600">
            Current state: {enabled1() ? "ON" : "OFF"}
          </div>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Switch States"
        description="Different switch states including disabled"
      >
        <div class="space-y-4">
          <div class="flex items-center space-x-3">
            <Switch checked={enabled2()} onChange={setEnabled2} />
            <span class="text-sm text-gray-700">Interactive switch</span>
          </div>

          <div class="flex items-center space-x-3">
            <Switch checked={true} disabled />
            <span class="text-sm text-gray-500">Disabled (ON)</span>
          </div>

          <div class="flex items-center space-x-3">
            <Switch checked={false} disabled />
            <span class="text-sm text-gray-500">Disabled (OFF)</span>
          </div>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Settings Panel Example"
        description="Switches used in a typical settings interface"
      >
        <div class="bg-white border rounded-lg p-4">
          <div class="space-y-4">
            <div class="flex items-center justify-between">
              <div>
                <div class="font-medium text-gray-900">Push Notifications</div>
                <div class="text-sm text-gray-500">
                  Receive notifications on your device
                </div>
              </div>
              <Switch checked={enabled1()} onChange={setEnabled1} />
            </div>

            <div class="flex items-center justify-between">
              <div>
                <div class="font-medium text-gray-900">Auto-save</div>
                <div class="text-sm text-gray-500">
                  Automatically save changes
                </div>
              </div>
              <Switch checked={enabled2()} onChange={setEnabled2} />
            </div>

            <div class="flex items-center justify-between">
              <div>
                <div class="font-medium text-gray-900">Analytics</div>
                <div class="text-sm text-gray-500">
                  Help improve our service
                </div>
              </div>
              <Switch checked={enabled3()} onChange={setEnabled3} />
            </div>
          </div>
        </div>
      </ComponentDemo>
    </div>
  )
}
