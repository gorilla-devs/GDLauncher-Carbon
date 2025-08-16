import { createFileRoute } from "@tanstack/solid-router"
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator
} from "../../../../src"
import ComponentDemo from "../../components/ComponentDemo"

export const Route = createFileRoute("/components/dropdownmenu")({
  component: DropdownMenuPage
})

function DropdownMenuPage() {
  return (
    <div class="max-w-4xl">
      <div class="mb-8">
        <h1
          class="text-4xl font-bold mb-4"
          style={`color: rgb(var(--lightSlate-50))`}
        >
          DropdownMenu
        </h1>
        <p class="text-xl" style={`color: rgb(var(--lightSlate-300))`}>
          Dropdown menu component that combines a trigger button with a
          contextual menu.
        </p>
      </div>

      <ComponentDemo
        title="Basic DropdownMenu"
        description="Simple dropdown menu with common actions"
      >
        <div class="flex justify-center">
          <DropdownMenu>
            <DropdownMenuTrigger>
              <button
                class="px-4 py-2 rounded-md transition-colors"
                style={`background-color: rgb(var(--primary-500)); color: white`}
              >
                Options ▼
              </button>
            </DropdownMenuTrigger>
            <DropdownMenuContent>
              <DropdownMenuItem onSelect={() => alert("Edit")}>
                Edit
              </DropdownMenuItem>
              <DropdownMenuItem onSelect={() => alert("Delete")}>
                Delete
              </DropdownMenuItem>
              <DropdownMenuItem onSelect={() => alert("Share")}>
                Share
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="DropdownMenu with Separators"
        description="Organized menu with separators between different action groups"
      >
        <div class="flex justify-center">
          <DropdownMenu>
            <DropdownMenuTrigger>
              <button
                class="px-4 py-2 rounded-md transition-colors"
                style={`background-color: rgb(var(--darkSlate-600)); color: rgb(var(--lightSlate-50))`}
              >
                File ▼
              </button>
            </DropdownMenuTrigger>
            <DropdownMenuContent>
              <DropdownMenuItem onSelect={() => alert("New")}>
                New
              </DropdownMenuItem>
              <DropdownMenuItem onSelect={() => alert("Open")}>
                Open
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem onSelect={() => alert("Save")}>
                Save
              </DropdownMenuItem>
              <DropdownMenuItem onSelect={() => alert("Save As")}>
                Save As
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem onSelect={() => alert("Print")}>
                Print
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="DropdownMenu with Icons"
        description="Menu items enhanced with icons for better UX"
      >
        <div class="flex justify-center">
          <DropdownMenu>
            <DropdownMenuTrigger>
              <button
                class="px-4 py-2 rounded-md transition-colors"
                style={`background-color: rgb(var(--green-600)); color: white`}
              >
                👤 User Menu ▼
              </button>
            </DropdownMenuTrigger>
            <DropdownMenuContent>
              <DropdownMenuItem onSelect={() => alert("Profile")}>
                👤 Profile
              </DropdownMenuItem>
              <DropdownMenuItem onSelect={() => alert("Settings")}>
                ⚙️ Settings
              </DropdownMenuItem>
              <DropdownMenuItem onSelect={() => alert("Help")}>
                ❓ Help
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem onSelect={() => alert("Logout")}>
                💪 Logout
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Disabled Menu Items"
        description="Some menu items can be disabled based on context"
      >
        <div class="flex justify-center">
          <DropdownMenu>
            <DropdownMenuTrigger>
              <button
                class="px-4 py-2 rounded-md transition-colors"
                style={`background-color: rgb(var(--darkSlate-600)); color: rgb(var(--lightSlate-50))`}
              >
                Edit ▼
              </button>
            </DropdownMenuTrigger>
            <DropdownMenuContent>
              <DropdownMenuItem onSelect={() => alert("Cut")}>
                Cut
              </DropdownMenuItem>
              <DropdownMenuItem onSelect={() => alert("Copy")}>
                Copy
              </DropdownMenuItem>
              <DropdownMenuItem disabled>Paste</DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem disabled>Delete</DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </ComponentDemo>
    </div>
  )
}
