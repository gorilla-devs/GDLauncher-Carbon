import { createFileRoute } from "@tanstack/solid-router"
import {
  ContextMenu,
  ContextMenuTrigger,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator
} from "../../../../src"
import ComponentDemo from "../../components/ComponentDemo"

export const Route = createFileRoute("/components/menu")({
  component: MenuPage
})

function MenuPage() {
  return (
    <div class="max-w-4xl">
      <div class="mb-8">
        <h1
          class="text-4xl font-bold mb-4"
          style={`color: rgb(var(--lightSlate-50))`}
        >
          ContextMenu
        </h1>
        <p class="text-xl" style={`color: rgb(var(--lightSlate-300))`}>
          Right-click contextual menu component for actions and navigation
          options.
        </p>
      </div>

      <ComponentDemo
        title="Basic Context Menu"
        description="Right-click on the area below to open a context menu"
      >
        <div class="flex justify-center">
          <ContextMenu>
            <ContextMenuTrigger>
              <div
                class="p-8 border-2 border-dashed rounded-lg text-center cursor-context-menu"
                style={`border-color: rgb(var(--darkSlate-600)); color: rgb(var(--lightSlate-300))`}
              >
                Right-click me to open context menu
              </div>
            </ContextMenuTrigger>
            <ContextMenuContent>
              <ContextMenuItem onSelect={() => alert("New File")}>
                New File
              </ContextMenuItem>
              <ContextMenuItem onSelect={() => alert("Open")}>
                Open
              </ContextMenuItem>
              <ContextMenuItem onSelect={() => alert("Save")}>
                Save
              </ContextMenuItem>
            </ContextMenuContent>
          </ContextMenu>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Context Menu with Separators"
        description="Organized menu with visual separators"
      >
        <div class="flex justify-center">
          <ContextMenu>
            <ContextMenuTrigger>
              <div
                class="p-8 border-2 border-dashed rounded-lg text-center cursor-context-menu"
                style={`border-color: rgb(var(--primary-500)); color: rgb(var(--lightSlate-300))`}
              >
                Right-click for edit menu
              </div>
            </ContextMenuTrigger>
            <ContextMenuContent>
              <ContextMenuItem onSelect={() => alert("Cut")}>
                Cut
              </ContextMenuItem>
              <ContextMenuItem onSelect={() => alert("Copy")}>
                Copy
              </ContextMenuItem>
              <ContextMenuItem onSelect={() => alert("Paste")}>
                Paste
              </ContextMenuItem>
              <ContextMenuSeparator />
              <ContextMenuItem onSelect={() => alert("Select All")}>
                Select All
              </ContextMenuItem>
              <ContextMenuSeparator />
              <ContextMenuItem onSelect={() => alert("Find")}>
                Find
              </ContextMenuItem>
            </ContextMenuContent>
          </ContextMenu>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Rich Context Menu"
        description="Context menu with various content types"
      >
        <div class="flex justify-center">
          <ContextMenu>
            <ContextMenuTrigger>
              <button
                class="px-6 py-3 rounded-md transition-colors cursor-context-menu"
                style={`background-color: rgb(var(--green-600)); color: white`}
              >
                Right-click this button
              </button>
            </ContextMenuTrigger>
            <ContextMenuContent>
              <ContextMenuItem onSelect={() => alert("Dashboard")}>
                📊 Dashboard
              </ContextMenuItem>
              <ContextMenuItem onSelect={() => alert("Profile")}>
                👤 Profile
              </ContextMenuItem>
              <ContextMenuItem onSelect={() => alert("Settings")}>
                ⚙️ Settings
              </ContextMenuItem>
              <ContextMenuSeparator />
              <ContextMenuItem onSelect={() => alert("Logout")}>
                💪 Logout
              </ContextMenuItem>
            </ContextMenuContent>
          </ContextMenu>
        </div>
      </ComponentDemo>
    </div>
  )
}
