import type { FEWorld } from "@gd/core_module/bindings"
import { JSX, Show, createEffect, createSignal, on, onCleanup } from "solid-js"
import { Trans } from "@gd/i18n"
import { worldLabel } from "@/utils/worldLabel"
import { renameTo } from "@/utils/instanceName"
import type { ChangesBlocked } from "@/pages/Library/shared/addons/components/ChangesBlocked"
import IconAction from "./IconAction"

export interface WorldTitleProps {
  world: FEWorld
  blocked: ChangesBlocked
  /** Why the world cannot be renamed now, when `blocked` does not say it. */
  reason?: JSX.Element
  /** Settles when the rename is done; throws when it was refused. */
  onRename: (_name: string) => Promise<void>
}

/** The selected world's name, with the folder beneath it when the two
 *  differ, and its inline rename. */
const WorldTitle = (props: WorldTitleProps) => {
  const label = () => worldLabel(props.world)
  const unreadable = () => props.world.identity === null
  const renameDisabled = () =>
    !!props.blocked || unreadable() || props.reason !== undefined

  const [editing, setEditing] = createSignal(false)
  const [draft, setDraft] = createSignal("")
  const [saving, setSaving] = createSignal(false)
  let box: HTMLDivElement | undefined
  let input: HTMLInputElement | undefined

  const close = () => setEditing(false)
  const start = () => {
    setDraft(label())
    setEditing(true)
    input?.focus()
    input?.select()
  }
  const save = async () => {
    if (saving()) return
    const name = renameTo(draft(), label())
    if (name === null) return close()
    setSaving(true)
    try {
      await props.onRename(name)
      close()
    } catch {
      // The global handler shows the refusal; the typed name stays.
    } finally {
      setSaving(false)
    }
  }
  const cancel = () => {
    if (!saving()) close()
  }

  // Another world shown in the header is not the one being renamed.
  createEffect(on(() => props.world.name, close, { defer: true }))

  createEffect(() => {
    if (!editing()) return
    const outside = (event: MouseEvent) => {
      if (!box?.contains(event.target as Node)) cancel()
    }
    document.addEventListener("mousedown", outside)
    onCleanup(() => document.removeEventListener("mousedown", outside))
  })

  return (
    <div class="flex min-w-0 flex-1 flex-col">
      <Show
        when={editing()}
        fallback={
          <div class="flex min-w-0 items-center gap-2">
            <span
              data-testid="world-title"
              class="text-lightSlate-50 truncate font-medium"
            >
              {label()}
            </span>
            <IconAction
              testId="world-rename"
              icon="i-hugeicons:pencil-edit-01"
              label={<Trans key="content:_trn_worlds.rename" />}
              onClick={start}
              disabled={renameDisabled()}
              blocked={props.blocked}
              reason={
                unreadable() ? (
                  <Trans key="content:_trn_worlds.unreadable" />
                ) : (
                  props.reason
                )
              }
            />
          </div>
        }
      >
        <div ref={box} class="flex min-w-0 items-center gap-2">
          <input
            ref={input}
            data-testid="world-rename-input"
            class="border-darkSlate-600 bg-darkSlate-700 text-lightSlate-50 focus:border-primary-500 box-border h-6 min-w-0 flex-1 rounded-md border border-solid px-1 text-base font-medium leading-6 outline-none disabled:opacity-50"
            maxLength={64}
            value={draft()}
            disabled={saving()}
            onInput={(event) => setDraft(event.currentTarget.value)}
            on:keydown={(event) => {
              if (event.key === "Enter") {
                event.preventDefault()
                void save()
              } else if (event.key === "Escape") {
                // Only the field closes, not what holds the page.
                event.preventDefault()
                event.stopPropagation()
                cancel()
              }
            }}
          />
          <div
            data-testid="world-rename-save"
            data-disabled={saving()}
            class="i-hugeicons:tick-02 h-5 w-5 shrink-0 transition-colors"
            classList={{
              "text-lightSlate-700 cursor-not-allowed": saving(),
              "text-lightSlate-500 cursor-pointer hover:text-green-500":
                !saving()
            }}
            onClick={() => void save()}
          />
          <div
            data-testid="world-rename-cancel"
            data-disabled={saving()}
            class="i-hugeicons:cancel-01 h-5 w-5 shrink-0 transition-colors"
            classList={{
              "text-lightSlate-700 cursor-not-allowed": saving(),
              "text-lightSlate-500 cursor-pointer hover:text-red-500": !saving()
            }}
            onClick={cancel}
          />
        </div>
      </Show>
      <Show when={label() !== props.world.name}>
        <span
          data-testid="world-folder"
          class="text-lightSlate-700 truncate text-xs"
        >
          {props.world.name}
        </span>
      </Show>
    </div>
  )
}

export default WorldTitle
