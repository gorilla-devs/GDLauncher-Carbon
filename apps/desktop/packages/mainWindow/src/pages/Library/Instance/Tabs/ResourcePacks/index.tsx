import {
  Button,
  Checkbox,
  Select,
  SelectTrigger,
  SelectContent,
  SelectItem,
  SelectValue,
  Input
} from "@gd/ui"
import { For, Show } from "solid-js"
import { Trans, useTransContext } from "@gd/i18n"
import ResourcePack from "./ResourcePack"
import { PlaceholderGorilla } from "@/components/PlaceholderGorilla"

interface IResourcepack {
  title: string
  enabled: boolean
  mcversion: string
  modloaderVersion: string
  resourcePackVersion: string
}

const resourcePacks: IResourcepack[] = [
  {
    title: "Mods1",
    enabled: true,
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
    resourcePackVersion: "1.17"
  },
  {
    title: "Mods2",
    enabled: true,
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
    resourcePackVersion: "1.15"
  },
  {
    title: "Mods3",
    enabled: true,
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
    resourcePackVersion: "1.15"
  },
  {
    title: "Mods4",
    enabled: true,
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
    resourcePackVersion: "1.14"
  },
  {
    title: "Mods5",
    enabled: true,
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
    resourcePackVersion: "1.17"
  },
  {
    title: "Mods6",
    enabled: true,
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
    resourcePackVersion: "1.17"
  },
  {
    title: "Mods7",
    enabled: true,
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
    resourcePackVersion: "1.17"
  },
  {
    title: "Mods8",
    enabled: true,
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
    resourcePackVersion: "1.17"
  },
  {
    title: "Mods9",
    enabled: true,
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",

    resourcePackVersion: "1.17"
  },
  {
    title: "Mods8",
    enabled: true,
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
    resourcePackVersion: "1.17"
  },
  {
    title: "Mods9",
    enabled: true,
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
    resourcePackVersion: "1.17"
  },
  {
    title: "Mods8",
    enabled: true,
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
    resourcePackVersion: "1.17"
  },
  {
    title: "Mods9",
    enabled: true,
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
    resourcePackVersion: "1.17"
  },
  {
    title: "Mods8",
    enabled: true,
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
    resourcePackVersion: "1.17"
  },
  {
    title: "Mods9",
    enabled: true,
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
    resourcePackVersion: "1.17"
  }
]

const NoResourcePacks = () => {
  return (
    <div class="min-h-90 flex h-full w-full items-center justify-center">
      <div class="flex flex-col items-center justify-center gap-6 text-center">
        <PlaceholderGorilla
          size={10}
          variant="Artistic Gorilla - Painter Palette"
        />
        <p class="text-lightSlate-700 max-w-100">
          <Trans
            key="content:_trn_no_resource_packs_text"
            options={{
              defaultValue:
                "At the moment this modpack does not contain resource packs, but you can add packs yourself from your folder"
            }}
          />
        </p>
        <Button type="outline" size="medium">
          <Trans
            key="content:_trn_add_resource_pack"
            options={{
              defaultValue: "+ Add pack"
            }}
          />
        </Button>
      </div>
    </div>
  )
}

const ResourcePacks = () => {
  const [t] = useTransContext()
  return (
    <div>
      <div class="bg-darkSlate-800 top-30 sticky z-10 flex flex-col pt-10 transition-all duration-100 ease-spring">
        <div class="flex flex-wrap items-center justify-between gap-1 pb-4">
          <Input
            placeholder={t("general:_trn_type_here")}
            icon={<div class="i-hugeicons:search-01" />}
            class="text-lightSlate-700 w-full rounded-full"
            inputClass=""
          />
          <div class="flex items-center gap-3">
            <p class="text-lightSlate-700">
              <Trans
                key="content:_trn_sort_by"
                options={{
                  defaultValue: "Sort by:"
                }}
              />
            </p>
            <Select
              value={"asc"}
              options={["asc", "desc"]}
              itemComponent={(props) => (
                <SelectItem item={props.item}>
                  {props.item.rawValue === "asc"
                    ? t("content:_trn_sort_by_asc")
                    : t("content:_trn_sort_by_desc")}
                </SelectItem>
              )}
            >
              <SelectTrigger variant="unstyled">
                <SelectValue<string>>
                  {(state) =>
                    state.selectedOption() === "asc"
                      ? t("content:_trn_sort_by_asc")
                      : t("content:_trn_sort_by_desc")
                  }
                </SelectValue>
              </SelectTrigger>
              <SelectContent />
            </Select>
          </div>
          <Button type="outline" size="medium">
            <Trans
              key="content:_trn_add_resource_pack"
              options={{
                defaultValue: "+ Add ResourcePack"
              }}
            />
          </Button>
        </div>
        <div class="text-lightSlate-700 z-10 mb-6 flex justify-between">
          <div class="flex gap-4">
            <div class="flex cursor-pointer items-center gap-2">
              <Checkbox checked={true} disabled={false} />
              <Trans
                key="content:_trn_select_all_resource_pack"
                options={{
                  defaultValue: "Select All"
                }}
              />
            </div>
            <div class="hover:text-lightSlate-50 flex cursor-pointer items-center gap-2 transition duration-100 ease-spring">
              <div class="i-hugeicons:folder-open text-2xl" />
              <Trans
                key="content:_trn_open_resource_packs_folder"
                options={{
                  defaultValue: "Open folder"
                }}
              />
            </div>
            <div class="hover:text-lightSlate-50 flex cursor-pointer items-center gap-2 transition duration-100 ease-spring">
              <div class="i-hugeicons:unavailable text-2xl" />
              <Trans
                key="content:_trn_disable_resource_pack"
                options={{
                  defaultValue: "disable"
                }}
              />
            </div>
            <div class="hover:text-lightSlate-50 flex cursor-pointer items-center gap-2 transition duration-100 ease-spring">
              <div class="i-hugeicons:delete-02 text-2xl" />
              <Trans
                key="content:_trn_delete_resource_pack"
                options={{
                  defaultValue: "delete"
                }}
              />
            </div>
          </div>
          <div>
            {resourcePacks.length}
            <Trans
              key="content:_trn_resource_packs"
              options={{
                defaultValue: "Resource packs"
              }}
            />
          </div>
        </div>
      </div>
      <div class="h-full overflow-y-hidden">
        <Show when={resourcePacks.length > 0} fallback={<NoResourcePacks />}>
          <For each={resourcePacks}>
            {(props) => <ResourcePack resourcePack={props} />}
          </For>
        </Show>
      </div>
    </div>
  )
}

export default ResourcePacks
