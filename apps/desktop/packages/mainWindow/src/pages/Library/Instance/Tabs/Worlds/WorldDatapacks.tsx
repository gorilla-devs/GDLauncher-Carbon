import type { FEWorld, FEWorldDatapack, Mod } from "@gd/core_module/bindings"
import {
  Button,
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
  Spinner,
  Tooltip,
  TooltipContent,
  TooltipTrigger
} from "@gd/ui"
import { For, JSX, Show } from "solid-js"
import { Trans } from "@gd/i18n"
import CurseforgeLogo from "/assets/images/icons/curseforge_logo.svg"
import ModrinthLogo from "/assets/images/icons/modrinth_logo.svg"
import { getWorldDatapackImageUrl } from "@/utils/instances"
import { worldLabel } from "@/utils/worldLabel"
import {
  BlockedText,
  type ChangesBlocked
} from "@/pages/Library/shared/addons/components/ChangesBlocked"
import { UpdateCopiesMenu } from "@/pages/Library/shared/addons/components/UpdateCopiesMenu"

export interface WorldDatapacksActions {
  openFolder: () => void
  deleteWorld: () => void
  addDatapacks: () => void
  copyName: () => void
  updateWorld: (_row: Mod) => void
  viewDetails: (_row: Mod) => void
  switchWorldVersion: (_row: Mod) => void
  openInApp: (_platform: "curseforge" | "modrinth") => void
  openInBrowser: (_platform: "curseforge" | "modrinth") => void
  updateHere: (_pack: FEWorldDatapack) => void
  updateEverywhere: (_pack: FEWorldDatapack) => void
  switchVersion: (_pack: FEWorldDatapack) => void
  remove: (_pack: FEWorldDatapack) => void
}

export interface WorldDatapacksProps {
  instanceId: number
  world: FEWorld
  /** The world's row in the instance's addon list, once that has loaded. */
  worldRow: Mod | undefined
  blocked: ChangesBlocked
  copiesWithUpdate: (_pack: FEWorldDatapack) => number
  isPending: (_packId: string) => boolean
  actions: WorldDatapacksActions
}

/** A row or header action drawn as an icon, as the Addons table draws them. */
const IconAction = (props: {
  icon: string
  label: JSX.Element
  onClick: () => void
  disabled?: boolean
  blocked?: ChangesBlocked
  danger?: boolean
  testId: string
}) => (
  <Tooltip>
    <TooltipTrigger>
      <div
        data-testid={props.testId}
        data-disabled={!!props.disabled}
        class={`${props.icon} h-5 w-5 transition-colors`}
        classList={{
          "text-lightSlate-700 cursor-not-allowed": !!props.disabled,
          "text-lightSlate-500 cursor-pointer": !props.disabled,
          "hover:text-red-400": !props.disabled && !!props.danger,
          "hover:text-lightSlate-200": !props.disabled && !props.danger
        }}
        onClick={() => {
          if (!props.disabled) props.onClick()
        }}
      />
    </TooltipTrigger>
    <TooltipContent>
      <Show when={props.disabled && props.blocked} fallback={props.label}>
        <BlockedText blocked={props.blocked!} />
      </Show>
    </TooltipContent>
  </Tooltip>
)

const packImage = (instanceId: number, pack: FEWorldDatapack) => {
  if (pack.curseforge?.has_image)
    return getWorldDatapackImageUrl(instanceId, pack.id, "curseforge")
  if (pack.modrinth?.has_image)
    return getWorldDatapackImageUrl(instanceId, pack.id, "modrinth")
  return null
}

const UpdateCell = (props: {
  pack: FEWorldDatapack
  pending: boolean
  blocked: ChangesBlocked
  copies: number
  actions: WorldDatapacksActions
}) => {
  const arrow = (onClick?: () => void) => (
    <div
      data-testid="world-datapack-update"
      data-disabled={!!props.blocked}
      class="i-hugeicons:download-02 text-lg transition-colors"
      classList={{
        "text-green-500 hover:text-green-400 cursor-pointer": !props.blocked,
        "text-gray-400 cursor-not-allowed": !!props.blocked
      }}
      onClick={() => onClick?.()}
    />
  )
  return (
    <Show
      when={!props.pending}
      fallback={<Spinner class="h-5 w-5 text-blue-400" />}
    >
      <Show when={props.pack.has_update}>
        <Show
          when={!props.blocked && props.copies > 1}
          fallback={
            <Tooltip>
              <TooltipTrigger>
                {arrow(() => {
                  if (!props.blocked) props.actions.updateHere(props.pack)
                })}
              </TooltipTrigger>
              <TooltipContent>
                <Show
                  when={props.blocked}
                  fallback={<Trans key="content:_trn_update_mod" />}
                >
                  <BlockedText blocked={props.blocked} />
                </Show>
              </TooltipContent>
            </Tooltip>
          }
        >
          <UpdateCopiesMenu
            copies={props.copies}
            testId="world-datapack-update"
            onUpdateHere={() => props.actions.updateHere(props.pack)}
            onUpdateEverywhere={() =>
              props.actions.updateEverywhere(props.pack)
            }
          >
            {arrow()}
          </UpdateCopiesMenu>
        </Show>
      </Show>
    </Show>
  )
}

const WorldDatapacks = (props: WorldDatapacksProps) => {
  const readable = () => props.world.identity !== null
  const downloaded = () =>
    !!(props.worldRow?.curseforge || props.worldRow?.modrinth)

  return (
    <div class="flex min-w-0 flex-1 flex-col overflow-hidden">
      <div class="box-border flex shrink-0 items-center gap-3 px-4 py-3">
        <div class="flex min-w-0 flex-1 flex-col">
          <span
            data-testid="world-title"
            class="text-lightSlate-50 truncate font-medium"
          >
            {worldLabel(props.world)}
          </span>
          <Show when={worldLabel(props.world) !== props.world.name}>
            <span
              data-testid="world-folder"
              class="text-lightSlate-700 truncate text-xs"
            >
              {props.world.name}
            </span>
          </Show>
        </div>
        <IconAction
          testId="world-open-folder"
          icon="i-hugeicons:folder-open"
          label={<Trans key="instances:_trn_open_folder" />}
          onClick={props.actions.openFolder}
        />
        <IconAction
          testId="world-delete"
          icon="i-hugeicons:delete-02"
          label={<Trans key="content:_trn_delete_mod" />}
          onClick={props.actions.deleteWorld}
          // The game has the save open and writes it when it closes.
          disabled={!!props.blocked}
          blocked={props.blocked}
          danger
        />
        <DropdownMenu placement="bottom-end">
          <Tooltip>
            <TooltipTrigger as="div">
              <DropdownMenuTrigger
                data-testid="world-more"
                class="b-0 bg-transparent p-0"
              >
                <div class="i-hugeicons:more-horizontal text-lightSlate-500 hover:text-lightSlate-200 h-5 w-5 cursor-pointer transition-colors" />
              </DropdownMenuTrigger>
            </TooltipTrigger>
            <TooltipContent>
              <Trans key="instances:_trn_more_actions" />
            </TooltipContent>
          </Tooltip>
          <DropdownMenuContent>
            <DropdownMenuItem
              data-testid="world-copy-name"
              onSelect={props.actions.copyName}
            >
              <div class="flex items-center gap-2">
                <div class="i-hugeicons:clipboard" />
                <Trans key="content:_trn_copy_name" />
              </div>
            </DropdownMenuItem>
            <Show when={props.worldRow?.has_update && props.worldRow}>
              {(row) => (
                <DropdownMenuItem
                  data-testid="world-update"
                  disabled={!!props.blocked}
                  onSelect={() => props.actions.updateWorld(row())}
                >
                  <div class="flex items-center gap-2">
                    <div class="i-hugeicons:download-02" />
                    <Trans key="content:_trn_update_mod" />
                  </div>
                </DropdownMenuItem>
              )}
            </Show>
            <Show when={downloaded() && props.worldRow}>
              {(row) => (
                <>
                  <DropdownMenuItem
                    data-testid="world-view-details"
                    onSelect={() => props.actions.viewDetails(row())}
                  >
                    <div class="flex items-center gap-2">
                      <div class="i-hugeicons:information-circle" />
                      <Trans key="content:_trn_view_details" />
                    </div>
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    data-testid="world-switch-version"
                    onSelect={() => props.actions.switchWorldVersion(row())}
                  >
                    <div class="flex items-center gap-2">
                      <div class="i-hugeicons:arrow-left-right" />
                      <Trans key="instances:_trn_switch_version" />
                    </div>
                  </DropdownMenuItem>
                  <Show when={row().curseforge}>
                    <DropdownMenuItem
                      onSelect={() => props.actions.openInApp("curseforge")}
                    >
                      <div class="flex items-center gap-2">
                        <img src={CurseforgeLogo} class="h-4 w-4" alt="" />
                        <Trans key="content:_trn_curseforge_open_in_app" />
                      </div>
                    </DropdownMenuItem>
                    <DropdownMenuItem
                      onSelect={() => props.actions.openInBrowser("curseforge")}
                    >
                      <div class="flex items-center gap-2">
                        <img src={CurseforgeLogo} class="h-4 w-4" alt="" />
                        <Trans key="content:_trn_curseforge_open_in_browser" />
                      </div>
                    </DropdownMenuItem>
                  </Show>
                  <Show when={row().modrinth}>
                    <DropdownMenuItem
                      onSelect={() => props.actions.openInApp("modrinth")}
                    >
                      <div class="flex items-center gap-2">
                        <img src={ModrinthLogo} class="h-4 w-4" alt="" />
                        <Trans key="content:_trn_modrinth_open_in_app" />
                      </div>
                    </DropdownMenuItem>
                    <DropdownMenuItem
                      onSelect={() => props.actions.openInBrowser("modrinth")}
                    >
                      <div class="flex items-center gap-2">
                        <img src={ModrinthLogo} class="h-4 w-4" alt="" />
                        <Trans key="content:_trn_modrinth_open_in_browser" />
                      </div>
                    </DropdownMenuItem>
                  </Show>
                </>
              )}
            </Show>
          </DropdownMenuContent>
        </DropdownMenu>
        <Show when={readable()}>
          <Tooltip>
            <TooltipTrigger as="div">
              <Button
                data-testid="world-add-datapacks"
                size="small"
                disabled={!!props.blocked}
                onClick={props.actions.addDatapacks}
              >
                <Trans key="content:_trn_worlds.add_datapacks" />
              </Button>
            </TooltipTrigger>
            <Show when={props.blocked}>
              <TooltipContent>
                <BlockedText blocked={props.blocked} />
              </TooltipContent>
            </Show>
          </Tooltip>
        </Show>
      </div>

      <Show
        when={readable()}
        fallback={
          <p
            data-testid="world-unreadable"
            class="text-lightSlate-600 m-0 px-4 text-sm"
          >
            <Trans key="content:_trn_worlds.unreadable" />
          </p>
        }
      >
        <div class="text-lightSlate-600 border-darkSlate-600 border-b-solid box-border flex shrink-0 items-center gap-3 border-0 border-b px-4 py-2 text-xs">
          <span class="min-w-0 flex-1">
            <Trans key="content:_trn_worlds.datapack" />
          </span>
          <span class="w-16 text-center">
            <Trans key="content:_trn_table.update" />
          </span>
          <span class="w-16 text-right">
            <Trans key="content:_trn_table.actions" />
          </span>
        </div>
        <div class="flex-1 overflow-y-auto">
          <Show
            when={props.world.datapacks.length > 0}
            fallback={
              <p class="text-lightSlate-600 m-0 px-4 py-3 text-sm">
                <Trans key="content:_trn_worlds.no_datapacks" />
              </p>
            }
          >
            <For each={props.world.datapacks}>
              {(pack) => {
                const pending = () => props.isPending(pack.id)
                const recognised = () => pack.name !== null
                const image = () => packImage(props.instanceId, pack)
                return (
                  <div
                    data-testid="world-datapack"
                    data-pending={pending()}
                    class="border-darkSlate-700 border-b-solid box-border flex items-center gap-3 border-0 border-b px-4 py-2 transition-opacity"
                    classList={{ "opacity-50": pending() }}
                  >
                    <div class="bg-darkSlate-600 flex h-8 w-8 shrink-0 items-center justify-center overflow-hidden rounded">
                      <Show
                        when={image()}
                        fallback={
                          <div
                            class={`${pack.is_folder ? "i-hugeicons:folder-01" : "i-hugeicons:file-01"} text-lightSlate-400 text-lg`}
                          />
                        }
                      >
                        <img
                          src={image()!}
                          class="h-full w-full object-cover"
                          alt=""
                          loading="lazy"
                        />
                      </Show>
                    </div>
                    <div class="flex min-w-0 flex-1 flex-col">
                      <span
                        data-testid="world-datapack-name"
                        class="text-lightSlate-100 truncate text-sm font-medium"
                      >
                        {pack.name ?? pack.filename}
                      </span>
                      <Show when={recognised()}>
                        <span
                          data-testid="world-datapack-filename"
                          class="text-lightSlate-600 truncate text-xs"
                        >
                          {pack.filename}
                        </span>
                      </Show>
                    </div>
                    <div class="flex w-16 shrink-0 items-center justify-center">
                      <UpdateCell
                        pack={pack}
                        pending={pending()}
                        blocked={props.blocked}
                        copies={props.copiesWithUpdate(pack)}
                        actions={props.actions}
                      />
                    </div>
                    <div class="flex w-16 shrink-0 items-center justify-end gap-3">
                      <Show when={recognised() && !pack.is_folder}>
                        <IconAction
                          testId="world-datapack-switch"
                          icon="i-hugeicons:arrow-left-right"
                          label={<Trans key="instances:_trn_switch_version" />}
                          onClick={() => props.actions.switchVersion(pack)}
                          disabled={!!props.blocked || pending()}
                          blocked={props.blocked}
                        />
                      </Show>
                      <IconAction
                        testId="world-datapack-remove"
                        icon="i-hugeicons:delete-02"
                        label={<Trans key="content:_trn_delete_mod" />}
                        onClick={() => props.actions.remove(pack)}
                        disabled={!!props.blocked || pending()}
                        blocked={props.blocked}
                        danger
                      />
                    </div>
                  </div>
                )
              }}
            </For>
          </Show>
        </div>
      </Show>
    </div>
  )
}

export default WorldDatapacks
