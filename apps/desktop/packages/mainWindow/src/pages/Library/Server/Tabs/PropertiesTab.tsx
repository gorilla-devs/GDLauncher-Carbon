import { createSignal, createEffect, For, Show, createMemo } from "solid-js"
import { useParams } from "@solidjs/router"
import { Button, Input, Tabs, TabsList, TabsTrigger, TabsContent, TabsIndicator } from "@gd/ui"
import { rspc } from "@/utils/rspcClient"
import useServerData from "../server.data"
import { propertyGroups, type PropertyDefinition } from "../Properties/propertyDefinitions"
import PropertyField from "../Properties/PropertyField"

const PropertiesTab = () => {
  const params = useParams()
  const routeData = useServerData()
  const serverId = () => parseInt(params.id, 10)

  const [properties, setProperties] = createSignal<Record<string, string>>({})
  const [originalProperties, setOriginalProperties] = createSignal<Record<string, string>>({})
  const [dirty, setDirty] = createSignal(false)
  const [activeGroup, setActiveGroup] = createSignal("gameplay")
  const [showRaw, setShowRaw] = createSignal(false)
  const [rawText, setRawText] = createSignal("")

  const propertiesQuery = rspc.createQuery(() => ({
    queryKey: ["server.getServerProperties", serverId()]
  }))

  const updatePropertiesMutation = rspc.createMutation(() => ({
    mutationKey: ["server.updateServerProperties"]
  }))

  // Load properties from query
  createEffect(() => {
    const data = propertiesQuery.data
    if (data) {
      setProperties({ ...data })
      setOriginalProperties({ ...data })
      setDirty(false)
    }
  })

  const isRunning = () => routeData.serverDetails.data?.state?.status === "running"

  const updateProperty = (key: string, value: string) => {
    setProperties((prev) => ({ ...prev, [key]: value }))
    setDirty(true)
  }

  const handleSave = async () => {
    // Only send changed properties
    const current = properties()
    const original = originalProperties()
    const changes: Record<string, string> = {}
    for (const key of Object.keys(current)) {
      if (current[key] !== original[key]) {
        changes[key] = current[key]
      }
    }
    // Also include new keys
    for (const key of Object.keys(current)) {
      if (!(key in original)) {
        changes[key] = current[key]
      }
    }

    if (Object.keys(changes).length > 0) {
      await updatePropertiesMutation.mutateAsync({
        id: serverId(),
        properties: changes
      })
      propertiesQuery.refetch()
    }
  }

  const handleReset = () => {
    setProperties({ ...originalProperties() })
    setDirty(false)
  }

  const handleRawSave = () => {
    const lines = rawText().split("\n")
    const newProps: Record<string, string> = {}
    for (const line of lines) {
      const trimmed = line.trim()
      if (!trimmed || trimmed.startsWith("#")) continue
      const idx = trimmed.indexOf("=")
      if (idx > 0) {
        newProps[trimmed.substring(0, idx).trim()] = trimmed.substring(idx + 1).trim()
      }
    }
    setProperties(newProps)
    setDirty(true)
    setShowRaw(false)
  }

  const currentGroupDefs = createMemo(() => {
    const group = propertyGroups.find((g) => g.id === activeGroup())
    return group?.properties ?? []
  })

  return (
    <div class="h-full w-full overflow-y-auto">
      <Show when={isRunning()}>
        <div class="mb-4 flex items-center gap-3 rounded-xl border border-yellow-600/30 bg-yellow-900/20 p-3">
          <div class="i-hugeicons:alert-01 text-xl text-yellow-500" />
          <span class="text-sm text-yellow-300/70">
            Some changes require a server restart to take effect.
          </span>
        </div>
      </Show>

      <div class="flex items-center justify-between mb-4">
        <div class="flex items-center gap-2">
          <Button
            size="small"
            type={showRaw() ? "primary" : "secondary"}
            onClick={() => {
              if (!showRaw()) {
                // Convert to raw text
                const text = Object.entries(properties())
                  .map(([k, v]) => `${k}=${v}`)
                  .join("\n")
                setRawText(text)
              }
              setShowRaw(!showRaw())
            }}
          >
            <div class="i-hugeicons:source-code h-4 w-4" />
            Raw Editor
          </Button>
        </div>
        <div class="flex items-center gap-2">
          <Show when={dirty()}>
            <Button size="small" type="secondary" onClick={handleReset}>
              Reset
            </Button>
            <Button
              size="small"
              type="primary"
              onClick={handleSave}
              loading={updatePropertiesMutation.isPending}
            >
              Save Changes
            </Button>
          </Show>
        </div>
      </div>

      <Show
        when={!showRaw()}
        fallback={
          <div class="flex flex-col gap-4">
            <textarea
              class="h-96 w-full rounded-xl border border-darkSlate-600 bg-darkSlate-900 p-4 font-mono text-sm text-lightSlate-200 outline-none"
              value={rawText()}
              onInput={(e) => setRawText(e.currentTarget.value)}
            />
            <div class="flex justify-end gap-2">
              <Button size="small" type="secondary" onClick={() => setShowRaw(false)}>
                Cancel
              </Button>
              <Button size="small" type="primary" onClick={handleRawSave}>
                Apply
              </Button>
            </div>
          </div>
        }
      >
        <div class="flex gap-4">
          {/* Category sidebar */}
          <div class="flex w-40 flex-shrink-0 flex-col gap-1">
            <For each={propertyGroups}>
              {(group) => (
                <button
                  class="flex items-center gap-2 rounded-lg px-3 py-2 text-left text-sm transition-colors"
                  classList={{
                    "bg-primary-500/20 text-primary-400": activeGroup() === group.id,
                    "text-lightSlate-500 hover:bg-darkSlate-700 hover:text-lightSlate-300": activeGroup() !== group.id
                  }}
                  onClick={() => setActiveGroup(group.id)}
                >
                  <div class={`h-4 w-4 ${group.icon}`} />
                  {group.label}
                </button>
              )}
            </For>
          </div>

          {/* Properties grid */}
          <div class="flex-1 rounded-xl border border-darkSlate-600 bg-darkSlate-900 p-4">
            <div class="flex flex-col gap-4">
              <For each={currentGroupDefs()}>
                {(prop) => (
                  <PropertyField
                    definition={prop}
                    value={properties()[prop.key] ?? prop.defaultValue}
                    onChange={(val) => updateProperty(prop.key, val)}
                  />
                )}
              </For>
            </div>
          </div>
        </div>
      </Show>
    </div>
  )
}

export default PropertiesTab
