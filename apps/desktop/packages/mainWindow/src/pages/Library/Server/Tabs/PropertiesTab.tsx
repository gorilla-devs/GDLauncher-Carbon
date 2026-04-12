import { createSignal, createEffect, createMemo, For, Show } from "solid-js"
import { useParams } from "@solidjs/router"
import {
  Button,
  Input,
  Select,
  SelectTrigger,
  SelectContent,
  SelectItem,
  SelectValue,
  Switch,
  Tooltip,
  TooltipTrigger,
  TooltipContent
} from "@gd/ui"
import { Trans, useTransContext, type NamespacedTranslationKey } from "@gd/i18n"
import { rspc } from "@/utils/rspcClient"
import useServerData from "../server.data"
import {
  propertyGroups,
  isAvailableForVersion,
  type PropertyDefinition
} from "../Properties/propertyDefinitions"
import Title from "@/pages/Settings/components/Title"
import Row from "@/pages/Settings/components/Row"
import RowsContainer from "@/pages/Settings/components/RowsContainer"
import RightHandSide from "@/pages/Settings/components/RightHandSide"

const GROUP_TRANSLATION_KEYS: Record<string, NamespacedTranslationKey> = {
  gameplay: "instances:_trn_server_properties_gameplay",
  world: "instances:_trn_server_properties_world",
  network: "instances:_trn_server_properties_network",
  performance: "instances:_trn_server_properties_performance",
  security: "instances:_trn_server_properties_security"
}

const PropertiesTab = () => {
  const [t] = useTransContext()
  const params = useParams()
  const routeData = useServerData()
  const serverId = () => parseInt(params.id!, 10)

  const [properties, setProperties] = createSignal<Record<string, string>>({})
  const [originalProperties, setOriginalProperties] = createSignal<
    Record<string, string>
  >({})
  const [dirty, setDirty] = createSignal(false)
  const [searchQuery, setSearchQuery] = createSignal("")

  const propertiesQuery = rspc.createQuery(() => ({
    queryKey: ["server.getServerProperties", serverId()]
  }))

  const updatePropertiesMutation = rspc.createMutation(() => ({
    mutationKey: ["server.updateServerProperties"]
  }))

  createEffect(() => {
    const data = propertiesQuery.data
    if (data) {
      setProperties({ ...data })
      setOriginalProperties({ ...data })
      setDirty(false)
    }
  })

  const isRunning = () =>
    routeData.serverDetails.data?.state?.status === "running"
  const gameVersion = () => routeData.serverDetails.data?.gameVersion

  const filteredGroups = createMemo(() => {
    const ver = gameVersion()
    const query = searchQuery().toLowerCase().trim()

    return propertyGroups
      .map((group) => ({
        ...group,
        properties: group.properties.filter((p) => {
          if (ver && !isAvailableForVersion(p, ver)) return false
          if (query) {
            return (
              p.label.toLowerCase().includes(query) ||
              p.key.toLowerCase().includes(query) ||
              p.description?.toLowerCase().includes(query)
            )
          }
          return true
        })
      }))
      .filter((group) => group.properties.length > 0)
  })

  const updateProperty = (key: string, value: string) => {
    setProperties((prev) => ({ ...prev, [key]: value }))
    setDirty(true)
  }

  const handleSave = async () => {
    const current = properties()
    const original = originalProperties()
    const changes: Record<string, string> = {}
    for (const key of Object.keys(current)) {
      if (current[key] !== original[key] || !(key in original)) {
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

  const renderField = (prop: PropertyDefinition) => {
    const value = () => properties()[prop.key] ?? prop.defaultValue

    return (
      <Row>
        <Title description={prop.description}>
          <span class="inline-flex items-center gap-1.5">
            {prop.label}
            <Show when={prop.info}>
              <Tooltip placement="top">
                <TooltipTrigger class="inline-flex items-center">
                  <div class="i-hugeicons:information-circle text-lightSlate-600 hover:text-lightSlate-400 h-4 w-4 shrink-0 cursor-help transition-colors" />
                </TooltipTrigger>
                <TooltipContent class="max-w-80">
                  <p class="m-0 text-xs leading-relaxed">{prop.info}</p>
                </TooltipContent>
              </Tooltip>
            </Show>
          </span>
        </Title>
        <RightHandSide>
          <Show when={prop.type === "boolean"}>
            <Switch
              checked={value() === "true"}
              onChange={(checked) =>
                updateProperty(prop.key, checked ? "true" : "false")
              }
            />
          </Show>
          <Show when={prop.type === "string"}>
            <Input
              class="w-60"
              value={value()}
              onInput={(e) => updateProperty(prop.key, e.currentTarget.value)}
            />
          </Show>
          <Show when={prop.type === "number"}>
            <Input
              class="w-40"
              type="number"
              value={value()}
              onInput={(e) => {
                const val = e.currentTarget.value
                if (prop.min !== undefined && Number(val) < prop.min) return
                if (prop.max !== undefined && Number(val) > prop.max) return
                updateProperty(prop.key, val)
              }}
            />
          </Show>
          <Show when={prop.type === "enum" && prop.enumValues}>
            <Select
              value={value()}
              onChange={(v) => {
                if (v) updateProperty(prop.key, v)
              }}
              options={prop.enumValues!}
              disallowEmptySelection={true}
              itemComponent={(itemProps) => {
                const idx = prop.enumValues!.indexOf(itemProps.item.rawValue)
                const label = prop.enumLabels?.[idx]
                return (
                  <SelectItem item={itemProps.item}>
                    {label || itemProps.item.rawValue}
                  </SelectItem>
                )
              }}
            >
              <SelectTrigger class="min-w-50">
                <SelectValue<string>>
                  {(state) => {
                    const idx = prop.enumValues!.indexOf(state.selectedOption())
                    return prop.enumLabels?.[idx] || state.selectedOption()
                  }}
                </SelectValue>
              </SelectTrigger>
              <SelectContent />
            </Select>
          </Show>
        </RightHandSide>
      </Row>
    )
  }

  return (
    <div class="w-full">
      <Show when={isRunning()}>
        <div class="mb-4 flex items-center gap-3 rounded-xl border border-yellow-600/30 bg-yellow-900/20 p-3">
          <div class="i-hugeicons:alert-01 text-xl text-yellow-500" />
          <span class="text-sm text-yellow-300/70">
            <Trans key="instances:_trn_server_properties_restart_warning" />
          </span>
        </div>
      </Show>

      <Show when={dirty()}>
        <div class="border-primary-600/30 bg-primary-900/20 sticky top-0 z-20 mb-4 flex items-center justify-between rounded-xl border p-3">
          <span class="text-lightSlate-300 text-sm">
            <Trans key="instances:_trn_server_properties_unsaved_changes" />
          </span>
          <div class="flex items-center gap-2">
            <Button size="small" type="secondary" onClick={handleReset}>
              <Trans key="instances:_trn_server_properties_reset" />
            </Button>
            <Button
              size="small"
              type="primary"
              onClick={handleSave}
              loading={updatePropertiesMutation.isPending}
            >
              <Trans key="instances:_trn_server_properties_save" />
            </Button>
          </div>
        </div>
      </Show>

      <div class="bg-darkSlate-800 sticky top-20 z-10 mb-4 pb-2 pt-2">
        <Input
          placeholder={
            t("instances:_trn_server_properties_search_placeholder") ||
            "Search properties..."
          }
          icon={<div class="i-hugeicons:search-01 h-4 w-4" />}
          value={searchQuery()}
          onInput={(e) => setSearchQuery(e.currentTarget.value)}
        />
      </div>

      <For each={filteredGroups()}>
        {(group) => (
          <div class="mb-2">
            <h3 class="text-lightSlate-100 mb-0 mt-8 flex items-center gap-2 text-xl font-medium">
              <div class={`h-5 w-5 ${group.icon}`} />
              {GROUP_TRANSLATION_KEYS[group.id]
                ? t(GROUP_TRANSLATION_KEYS[group.id])
                : group.label}
            </h3>
            <RowsContainer>
              <For each={group.properties}>{(prop) => renderField(prop)}</For>
            </RowsContainer>
          </div>
        )}
      </For>
    </div>
  )
}

export default PropertiesTab
