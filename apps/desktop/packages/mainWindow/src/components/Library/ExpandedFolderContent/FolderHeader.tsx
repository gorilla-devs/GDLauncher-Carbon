import { Accessor, For, Show } from "solid-js"
import {
  Button,
  Input,
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger
} from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import type { LibrarySortCriteria } from "@gd/core_module/bindings"

interface FolderHeaderProps {
  groupId: number
  groupName: string
  instanceCount: Accessor<number>
  isDefaultGroup: boolean
  isEditing: Accessor<boolean>
  editValue: Accessor<string>
  onStartEdit: () => void
  onSave: () => void
  onEditValueChange: (value: string) => void
  onKeyDown: (e: KeyboardEvent) => void
  onSort: (sortBy: LibrarySortCriteria) => void
  onClose: () => void
  viewTransitionName: string | undefined
  inputRef?: (el: HTMLInputElement) => void
}

const SORT_OPTIONS = [
  {
    sortBy: "name" as LibrarySortCriteria,
    icon: "i-hugeicons:text",
    key: "ui:_trn_by_name" as const
  },
  {
    sortBy: "lastPlayed" as LibrarySortCriteria,
    icon: "i-hugeicons:clock-01",
    key: "ui:_trn_by_last_played" as const
  },
  {
    sortBy: "mostPlayed" as LibrarySortCriteria,
    icon: "i-hugeicons:time-02",
    key: "ui:_trn_by_most_played" as const
  },
  {
    sortBy: "dateCreated" as LibrarySortCriteria,
    icon: "i-hugeicons:calendar-add-01",
    key: "ui:_trn_by_date_created" as const
  }
]

export function FolderHeader(props: FolderHeaderProps) {
  const [t] = useTransContext()

  return (
    <div class="mb-4 flex items-center justify-between">
      <div
        class="flex items-center gap-2"
        style={
          props.viewTransitionName
            ? { "view-transition-name": props.viewTransitionName }
            : {}
        }
      >
        <div class="i-hugeicons:folder-01 text-primary-400" />
        <Show
          when={!props.isEditing()}
          fallback={
            <Input
              ref={props.inputRef}
              value={props.editValue()}
              onInput={(e) => props.onEditValueChange(e.currentTarget.value)}
              onKeyDown={props.onKeyDown}
              onBlur={props.onSave}
              class="h-7 w-48 py-0 text-base"
            />
          }
        >
          <h3
            class="text-lightSlate-100 hover:text-lightSlate-50 cursor-pointer text-lg font-medium"
            classList={{
              "cursor-default": props.isDefaultGroup
            }}
            onDblClick={props.onStartEdit}
          >
            {props.groupName}
          </h3>
        </Show>
        <span class="text-darkSlate-400 text-sm">
          ({props.instanceCount()})
        </span>
      </div>
      <div class="flex items-center gap-2">
        {/* Sort dropdown */}
        <DropdownMenu>
          <DropdownMenuTrigger>
            <Button
              variant="ghost"
              size="small"
              title={t("instances:_trn_rearrange")}
            >
              <div class="i-hugeicons:arrow-up-down h-4 w-4" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent>
            <DropdownMenuLabel>
              <Trans key="instances:_trn_rearrange" />
            </DropdownMenuLabel>
            <DropdownMenuSeparator />
            <For each={SORT_OPTIONS}>
              {(option) => (
                <DropdownMenuItem onClick={() => props.onSort(option.sortBy)}>
                  <div class="flex items-center gap-2">
                    <div class={`${option.icon} h-4 w-4`} />
                    <Trans key={option.key} />
                  </div>
                </DropdownMenuItem>
              )}
            </For>
          </DropdownMenuContent>
        </DropdownMenu>
        <Show when={!props.isDefaultGroup}>
          <Button
            variant="ghost"
            size="small"
            onClick={props.onStartEdit}
            title={t("instances:_trn_rename_group")}
          >
            <div class="i-hugeicons:pencil-edit-01 h-4 w-4" />
          </Button>
        </Show>
        <Button variant="ghost" size="small" onClick={props.onClose}>
          <div class="i-hugeicons:cancel-01 h-4 w-4" />
        </Button>
      </div>
    </div>
  )
}
