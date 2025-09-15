import { createFileRoute } from "@tanstack/solid-router"
import { createSignal, For } from "solid-js"
import {
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem
} from "../../../../src"
import ComponentDemo from "../../components/ComponentDemo"

export const Route = createFileRoute("/components/select")({
  component: SelectPage
})

function SelectPage() {
  const [selectedVersion, setSelectedVersion] = createSignal("")
  const [selectedNativeVersion, setSelectedNativeVersion] = createSignal("")
  const [selectedModLoader, setSelectedModLoader] = createSignal("fabric")
  const [selectedModCategory, setSelectedModCategory] = createSignal("")
  const [selectedInstanceType, setSelectedInstanceType] = createSignal("modded")
  const [selectedJavaVersion, setSelectedJavaVersion] = createSignal("")
  const [selectedMemoryAllocation, setSelectedMemoryAllocation] =
    createSignal("")

  const minecraftVersions = ["1.21.4", "1.21.3", "1.21.1", "1.20.1", "1.19.2"]
  const modLoaders = [
    { value: "fabric", label: "Fabric", description: "Lightweight and fast" },
    { value: "forge", label: "Forge", description: "Most popular mod loader" },
    {
      value: "quilt",
      label: "Quilt",
      description: "Fabric fork with enhancements"
    },
    {
      value: "neoforge",
      label: "NeoForge",
      description: "Modern Forge alternative"
    },
    { value: "vanilla", label: "Vanilla", description: "No mods" }
  ]
  const modCategories = [
    {
      value: "technology",
      label: "Technology",
      color: "bg-blue-500",
      icon: "⚙️"
    },
    { value: "magic", label: "Magic", color: "bg-purple-500", icon: "✨" },
    {
      value: "adventure",
      label: "Adventure",
      color: "bg-green-500",
      icon: "🗡️"
    },
    {
      value: "decoration",
      label: "Decoration",
      color: "bg-yellow-500",
      icon: "🎨"
    },
    {
      value: "optimization",
      label: "Optimization",
      color: "bg-red-500",
      icon: "⚡"
    }
  ]

  const javaVersions = ["Java 8", "Java 11", "Java 17", "Java 21"]
  const memoryOptions = ["2GB", "4GB", "6GB", "8GB", "12GB", "16GB"]

  return (
    <div class="max-w-4xl">
      <div class="mb-8">
        <h1
          class="text-4xl font-bold mb-4"
          style={`color: rgb(var(--lightSlate-50))`}
        >
          Select
        </h1>
        <p class="text-xl" style={`color: rgb(var(--lightSlate-300))`}>
          A powerful dropdown selection component built with Kobalte primitives,
          now with a working implementation!
        </p>
      </div>

      <ComponentDemo
        title="Minecraft Version Selector"
        description="Select Minecraft version for your instance"
      >
        <div class="w-48">
          <Select
            options={minecraftVersions}
            value={selectedVersion()}
            onChange={setSelectedVersion}
            itemComponent={(props) => (
              <SelectItem item={props.item}>
                Minecraft {props.item.rawValue}
              </SelectItem>
            )}
          >
            <SelectTrigger>
              <SelectValue<string>>
                {(state) =>
                  state.selectedOption()
                    ? `Minecraft ${state.selectedOption()}`
                    : "Select version"
                }
              </SelectValue>
            </SelectTrigger>
            <SelectContent />
          </Select>
        </div>
        <p class="mt-2 text-sm" style={`color: rgb(var(--lightSlate-400))`}>
          Selected: {selectedVersion() || "None"}
        </p>
      </ComponentDemo>

      <ComponentDemo
        title="Native HTML Select Alternative"
        description="Fallback using styled HTML select (for comparison)"
      >
        <div class="w-48">
          <select
            value={selectedNativeVersion()}
            onChange={(e) => setSelectedNativeVersion(e.target.value)}
            class="w-full px-3 py-2 text-sm border rounded-md transition-colors focus:outline-none focus:ring-2 focus:ring-primary-500"
            style="background-color: rgb(var(--darkSlate-800)); border-color: rgb(var(--darkSlate-600)); color: rgb(var(--lightSlate-50))"
          >
            <option
              value=""
              style="background-color: rgb(var(--darkSlate-700))"
            >
              Select version
            </option>
            <For each={minecraftVersions}>
              {(version) => (
                <option
                  value={version}
                  style="background-color: rgb(var(--darkSlate-700)); color: rgb(var(--lightSlate-50))"
                >
                  Minecraft {version}
                </option>
              )}
            </For>
          </select>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Mod Loader Selector"
        description="Choose mod loader with descriptions and default selection"
      >
        <div class="w-64">
          <Select
            options={modLoaders.map((ml) => ml.value)}
            value={selectedModLoader()}
            onChange={setSelectedModLoader}
            itemComponent={(props) => {
              const modLoader = modLoaders.find(
                (ml) => ml.value === props.item.rawValue
              )
              return (
                <SelectItem item={props.item}>
                  <div class="flex flex-col">
                    <div class="font-medium">{modLoader?.label}</div>
                    <div class="text-xs opacity-75">
                      {modLoader?.description}
                    </div>
                  </div>
                </SelectItem>
              )
            }}
          >
            <SelectTrigger>
              <SelectValue<string>>
                {(state) => {
                  const modLoader = modLoaders.find(
                    (ml) => ml.value === state.selectedOption()
                  )
                  return modLoader?.label || "Choose mod loader"
                }}
              </SelectValue>
            </SelectTrigger>
            <SelectContent />
          </Select>
        </div>
        <p class="mt-2 text-sm" style={`color: rgb(var(--lightSlate-400))`}>
          Selected:{" "}
          {modLoaders.find((ml) => ml.value === selectedModLoader())?.label ||
            "None"}
        </p>
      </ComponentDemo>

      <ComponentDemo
        title="Mod Category Selector"
        description="Browse mods by category with visual indicators and icons"
      >
        <div class="w-64">
          <Select
            options={modCategories.map((cat) => cat.value)}
            value={selectedModCategory()}
            onChange={setSelectedModCategory}
            itemComponent={(props) => {
              const category = modCategories.find(
                (cat) => cat.value === props.item.rawValue
              )
              return (
                <SelectItem item={props.item}>
                  <div class="flex items-center gap-2">
                    <span class="text-base">{category?.icon}</span>
                    <div
                      class={`w-3 h-3 rounded-full ${category?.color}`}
                    ></div>
                    {category?.label}
                  </div>
                </SelectItem>
              )
            }}
          >
            <SelectTrigger>
              <SelectValue<string>>
                {(state) => {
                  const category = modCategories.find(
                    (cat) => cat.value === state.selectedOption()
                  )
                  return category ? (
                    <div class="flex items-center gap-2">
                      <span class="text-base">{category.icon}</span>
                      <div
                        class={`w-3 h-3 rounded-full ${category.color}`}
                      ></div>
                      {category.label}
                    </div>
                  ) : (
                    "Select mod category"
                  )
                }}
              </SelectValue>
            </SelectTrigger>
            <SelectContent />
          </Select>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Disabled Instance Settings"
        description="Select component in disabled state when instance is locked"
      >
        <div class="w-48">
          <Select
            options={["Creative", "Survival", "Adventure", "Spectator"]}
            value=""
            onChange={() => {}}
            disabled
            itemComponent={(props) => (
              <SelectItem item={props.item}>{props.item.rawValue}</SelectItem>
            )}
          >
            <SelectTrigger>
              <SelectValue<string>>
                {() => "Instance is running..."}
              </SelectValue>
            </SelectTrigger>
            <SelectContent />
          </Select>
        </div>
        <p class="mt-2 text-sm" style={`color: rgb(var(--lightSlate-400))`}>
          Settings are locked while the instance is running
        </p>
      </ComponentDemo>

      <ComponentDemo
        title="Custom Instance Type Selector"
        description="Highlighted select for creating new instances"
      >
        <div class="w-52">
          <Select
            options={["vanilla", "modded", "modpack", "snapshot"]}
            value={selectedInstanceType()}
            onChange={setSelectedInstanceType}
            itemComponent={(props) => {
              const getDescription = (type: string) => {
                switch (type) {
                  case "vanilla":
                    return "Pure Minecraft experience"
                  case "modded":
                    return "Custom mods collection"
                  case "modpack":
                    return "Pre-configured mod bundle"
                  case "snapshot":
                    return "Latest experimental features"
                  default:
                    return ""
                }
              }
              return (
                <SelectItem item={props.item}>
                  <div class="flex flex-col">
                    <div class="font-medium">
                      {props.item.rawValue.charAt(0).toUpperCase() +
                        props.item.rawValue.slice(1)}
                    </div>
                    <div class="text-xs opacity-75">
                      {getDescription(props.item.rawValue)}
                    </div>
                  </div>
                </SelectItem>
              )
            }}
          >
            <SelectTrigger
              variant="unstyled"
              class="w-full p-3 text-sm border-2 rounded-lg transition-colors focus:outline-none focus:ring-2 focus:ring-primary-400"
              style="background-color: rgba(var(--primary-500), 0.1); border-color: rgb(var(--primary-500)); color: rgb(var(--lightSlate-50))"
            >
              <SelectValue<string>>
                {(state) => {
                  const type = state.selectedOption()
                  return type
                    ? type.charAt(0).toUpperCase() + type.slice(1) + " Instance"
                    : "Choose instance type"
                }}
              </SelectValue>
              <svg
                xmlns="http://www.w3.org/2000/svg"
                width="1em"
                height="1em"
                viewBox="0 0 24 24"
                class="flex size-4 items-center justify-center opacity-50"
              >
                <path
                  fill="none"
                  stroke="currentColor"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="m8 9l4-4l4 4m0 6l-4 4l-4-4"
                />
              </svg>
            </SelectTrigger>
            <SelectContent />
          </Select>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Instance Configuration Form"
        description="Multiple launcher settings in a form layout"
      >
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4 w-full max-w-lg">
          <div class="space-y-2">
            <label
              class="text-sm font-medium"
              style={`color: rgb(var(--lightSlate-200))`}
            >
              Java Version
            </label>
            <Select
              options={javaVersions}
              value={selectedJavaVersion()}
              onChange={setSelectedJavaVersion}
              itemComponent={(props) => (
                <SelectItem item={props.item}>{props.item.rawValue}</SelectItem>
              )}
            >
              <SelectTrigger>
                <SelectValue<string>>
                  {(state) => state.selectedOption() || "Auto-detect"}
                </SelectValue>
              </SelectTrigger>
              <SelectContent />
            </Select>
          </div>

          <div class="space-y-2">
            <label
              class="text-sm font-medium"
              style={`color: rgb(var(--lightSlate-200))`}
            >
              Memory Allocation
            </label>
            <Select
              options={memoryOptions}
              value={selectedMemoryAllocation()}
              onChange={setSelectedMemoryAllocation}
              itemComponent={(props) => (
                <SelectItem item={props.item}>{props.item.rawValue}</SelectItem>
              )}
            >
              <SelectTrigger>
                <SelectValue<string>>
                  {(state) => state.selectedOption() || "4GB (Recommended)"}
                </SelectValue>
              </SelectTrigger>
              <SelectContent />
            </Select>
          </div>
        </div>
        <div
          class="mt-4 p-3 rounded-md"
          style="background-color: rgb(var(--darkSlate-700)); border: 1px solid rgb(var(--darkSlate-600))"
        >
          <div
            class="text-sm font-medium mb-1"
            style="color: rgb(var(--lightSlate-200))"
          >
            Configuration Summary:
          </div>
          <div class="text-sm" style="color: rgb(var(--lightSlate-400))">
            Java: {selectedJavaVersion() || "Auto-detect"} | Memory:{" "}
            {selectedMemoryAllocation() || "4GB (Default)"}
          </div>
        </div>
      </ComponentDemo>
    </div>
  )
}
