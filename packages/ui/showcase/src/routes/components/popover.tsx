import { createFileRoute } from "@tanstack/solid-router"
import {
  Popover,
  PopoverTrigger,
  PopoverContent,
  PopoverTitle,
  PopoverDescription
} from "../../../../src"
import ComponentDemo from "../../components/ComponentDemo"

export const Route = createFileRoute("/components/popover")({
  component: PopoverPage
})

function PopoverPage() {
  return (
    <div class="max-w-4xl">
      <div class="mb-8">
        <h1
          class="text-4xl font-bold mb-4"
          style={`color: rgb(var(--lightSlate-50))`}
        >
          Popover
        </h1>
        <p class="text-xl" style={`color: rgb(var(--lightSlate-300))`}>
          Floating content container that appears relative to a trigger element.
        </p>
      </div>

      <ComponentDemo
        title="Basic Popover"
        description="Simple popover triggered by click"
      >
        <div class="flex justify-center">
          <Popover>
            <PopoverTrigger>
              <button class="px-4 py-2 bg-indigo-600 hover:bg-indigo-700 text-white rounded-md transition-colors">
                Click me
              </button>
            </PopoverTrigger>
            <PopoverContent>
              <PopoverTitle class="font-semibold mb-2">
                Popover Title
              </PopoverTitle>
              <PopoverDescription class="text-sm">
                This is the popover content that appears when triggered.
              </PopoverDescription>
            </PopoverContent>
          </Popover>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Popover Positions"
        description="Popovers can appear in different positions"
      >
        <div class="grid grid-cols-2 gap-8 max-w-md mx-auto">
          <div class="flex justify-center">
            <Popover placement="top">
              <PopoverTrigger>
                <button class="px-4 py-2 bg-gray-600 hover:bg-gray-700 text-white rounded-md transition-colors">
                  Top
                </button>
              </PopoverTrigger>
              <PopoverContent>
                <div class="p-2">Top popover</div>
              </PopoverContent>
            </Popover>
          </div>

          <div class="flex justify-center">
            <Popover placement="bottom">
              <PopoverTrigger>
                <button class="px-4 py-2 bg-gray-600 hover:bg-gray-700 text-white rounded-md transition-colors">
                  Bottom
                </button>
              </PopoverTrigger>
              <PopoverContent>
                <div class="p-2">Bottom popover</div>
              </PopoverContent>
            </Popover>
          </div>

          <div class="flex justify-center">
            <Popover placement="left">
              <PopoverTrigger>
                <button class="px-4 py-2 bg-gray-600 hover:bg-gray-700 text-white rounded-md transition-colors">
                  Left
                </button>
              </PopoverTrigger>
              <PopoverContent>
                <div class="p-2">Left popover</div>
              </PopoverContent>
            </Popover>
          </div>

          <div class="flex justify-center">
            <Popover placement="right">
              <PopoverTrigger>
                <button class="px-4 py-2 bg-gray-600 hover:bg-gray-700 text-white rounded-md transition-colors">
                  Right
                </button>
              </PopoverTrigger>
              <PopoverContent>
                <div class="p-2">Right popover</div>
              </PopoverContent>
            </Popover>
          </div>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Rich Popover Content"
        description="Popover with complex interactive content"
      >
        <div class="flex justify-center">
          <Popover>
            <PopoverTrigger>
              <button class="px-4 py-2 bg-green-600 hover:bg-green-700 text-white rounded-md transition-colors">
                Settings Menu
              </button>
            </PopoverTrigger>
            <PopoverContent class="w-56">
              <PopoverTitle class="font-semibold mb-3">
                Settings Menu
              </PopoverTitle>
              <div class="space-y-1">
                <button class="block w-full text-left px-3 py-2 rounded text-sm hover:bg-gray-600 transition-colors">
                  Profile Settings
                </button>
                <button class="block w-full text-left px-3 py-2 rounded text-sm hover:bg-gray-600 transition-colors">
                  Preferences
                </button>
                <button class="block w-full text-left px-3 py-2 rounded text-sm hover:bg-gray-600 transition-colors">
                  Notifications
                </button>
                <hr class="border-gray-600 my-2" />
                <button class="block w-full text-left px-3 py-2 rounded text-sm text-red-400 hover:bg-gray-600 transition-colors">
                  Logout
                </button>
              </div>
            </PopoverContent>
          </Popover>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Custom Styled Popover"
        description="Popover with custom styling and no close button"
      >
        <div class="flex justify-center">
          <Popover>
            <PopoverTrigger>
              <button class="px-4 py-2 bg-purple-600 hover:bg-purple-700 text-white rounded-md transition-colors">
                Info Panel
              </button>
            </PopoverTrigger>
            <PopoverContent hideCloseButton class="w-80 p-6">
              <PopoverTitle class="text-lg font-bold mb-2">
                Information Panel
              </PopoverTitle>
              <PopoverDescription class="text-sm mb-4">
                This popover demonstrates custom styling and content layout
                without a close button.
              </PopoverDescription>
              <div class="space-y-2">
                <div class="flex justify-between text-sm">
                  <span class="text-gray-400">Status:</span>
                  <span class="text-green-400">Active</span>
                </div>
                <div class="flex justify-between text-sm">
                  <span class="text-gray-400">Version:</span>
                  <span>1.2.3</span>
                </div>
                <div class="flex justify-between text-sm">
                  <span class="text-gray-400">Last Updated:</span>
                  <span>2 hours ago</span>
                </div>
              </div>
            </PopoverContent>
          </Popover>
        </div>
      </ComponentDemo>
    </div>
  )
}
