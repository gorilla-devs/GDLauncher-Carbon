import {
  createMemo,
  createSignal,
  createUniqueId,
  Show,
  type Accessor,
  type Component,
  type JSX
} from "solid-js"
import { Input, Popover, PopoverContent, PopoverTrigger } from "@gd/ui"
import { useTransContext } from "@gd/i18n"
import type { ListInstance } from "@gd/core_module/bindings"
import type { VListHandle } from "virtua/lib/solid"
import { VList } from "@/components/VirtuaWrapper"
import { getInstanceImageUrl } from "@/utils/instances"
import DefaultImg from "/assets/images/default-instance-img.png"

/** An instance as a row of the picker. */
export type PickableInstance = {
  id: number
  name: string
  gameVersion: string
  modloader: string
  iconUrl: string | null
}

/** A library instance as a row of the picker. */
export function pickable(instance: ListInstance): PickableInstance {
  const valid =
    instance.status.status === "valid" ? instance.status.value : null
  return {
    id: instance.id,
    name: instance.name,
    gameVersion: valid?.mc_version || "",
    modloader: valid?.modloader || "vanilla",
    iconUrl: instance.icon_revision
      ? getInstanceImageUrl(instance.id, instance.icon_revision)
      : null
  }
}

/** The instances whose name, game version or loader holds what was typed. */
export function matching<T extends Omit<PickableInstance, "id" | "iconUrl">>(
  instances: readonly T[],
  typed: string
): readonly T[] {
  const query = typed.toLowerCase().trim()
  if (!query) return instances
  return instances.filter(
    (instance) =>
      instance.name.toLowerCase().includes(query) ||
      instance.gameVersion.toLowerCase().includes(query) ||
      instance.modloader.toLowerCase().includes(query)
  )
}

/** The row an arrow key moves to from `active`, in a list of `count`. */
export function moved(active: number, by: number, count: number): number {
  return Math.max(0, Math.min(count - 1, active + by))
}

/** The height of a row, and how many show before the list scrolls. */
const ROW = 48
const ROWS_SHOWN = 6

/**
 * A button that opens a list of instances to pick one from, for any number
 * of them: a box on top narrows the list by name, game version or loader as
 * it is typed in, and the list draws only the rows in view, so thousands
 * scroll like ten. The arrow keys and Enter pick from the box.
 *
 * `trigger` is the button and `children` what it shows; `label` says above
 * the list what picking does, where the button alone does not.
 */
export default function InstancePicker(props: {
  instances: Accessor<readonly PickableInstance[]>
  onPick: (id: number) => void
  trigger: Component<{ children?: JSX.Element; disabled?: boolean }>
  children: JSX.Element
  label?: string
  disabled?: boolean
}) {
  const [t] = useTransContext()
  const [open, setOpen] = createSignal(false)
  const [typed, setTyped] = createSignal("")
  const [active, setActive] = createSignal(0)
  const list = createUniqueId()
  let rows: VListHandle | undefined

  const found = createMemo(() => matching(props.instances(), typed()))

  const pick = (instance: PickableInstance | undefined) => {
    if (!instance) return
    setOpen(false)
    props.onPick(instance.id)
  }
  const move = (by: number) => {
    const next = moved(active(), by, found().length)
    setActive(next)
    rows?.scrollToIndex(next, { align: "nearest" })
  }

  return (
    <Popover
      open={open()}
      onOpenChange={(next) => {
        if (next) {
          setTyped("")
          setActive(0)
        }
        setOpen(next)
      }}
      placement="bottom-end"
    >
      <PopoverTrigger as={props.trigger} disabled={props.disabled}>
        {props.children}
      </PopoverTrigger>
      <PopoverContent
        class="w-80 p-0"
        hideCloseButton
        data-testid="instance-picker"
      >
        <Show when={props.label}>
          <div class="text-lightSlate-500 px-3 pt-2 text-xs font-semibold">
            {props.label}
          </div>
        </Show>
        <div class="border-darkSlate-500 h-12 border-0 border-b border-solid p-2">
          <Input
            data-testid="instance-picker-search"
            role="combobox"
            aria-expanded="true"
            aria-controls={list}
            aria-activedescendant={
              found().length > 0 ? `${list}-${active()}` : undefined
            }
            placeholder={t("search:_trn_search_instances")}
            value={typed()}
            onInput={(event) => {
              setTyped(event.currentTarget.value)
              setActive(0)
              rows?.scrollToIndex(0)
            }}
            onKeyDown={(event) => {
              if (event.key === "ArrowDown") move(1)
              else if (event.key === "ArrowUp") move(-1)
              else if (event.key === "PageDown") move(ROWS_SHOWN)
              else if (event.key === "PageUp") move(-ROWS_SHOWN)
              else if (event.key === "Enter") pick(found()[active()])
              else return
              event.preventDefault()
            }}
            icon={<div class="i-hugeicons:search-01 h-4 w-4" />}
            variant="transparent"
            class="h-full"
          />
        </div>
        <Show
          when={found().length > 0}
          fallback={
            <div
              class="text-lightSlate-400 px-2 py-3 text-center text-sm"
              data-testid="instance-picker-none"
            >
              {t("content:_trn_common.no_instances_found")}
            </div>
          }
        >
          <div
            id={list}
            role="listbox"
            style={{
              height: `${Math.min(found().length, ROWS_SHOWN) * ROW}px`
            }}
          >
            <VList
              ref={(handle) => (rows = handle)}
              data={found()}
              itemSize={ROW}
              class="h-full w-full"
            >
              {(instance, index) => (
                <div
                  id={`${list}-${index()}`}
                  role="option"
                  aria-selected={index() === active()}
                  data-testid="instance-picker-row"
                  class="flex cursor-pointer items-center gap-2 px-3"
                  classList={{ "bg-darkSlate-600": index() === active() }}
                  style={{ height: `${ROW}px` }}
                  onMouseMove={() => setActive(index())}
                  onClick={() => pick(instance)}
                >
                  <img
                    src={instance.iconUrl || DefaultImg}
                    alt=""
                    class="h-8 w-8 shrink-0 rounded object-cover"
                  />
                  <div class="min-w-0 flex-1">
                    <div
                      class="text-lightSlate-50 truncate text-sm font-medium"
                      data-testid="instance-picker-name"
                    >
                      {instance.name}
                    </div>
                    <div class="text-lightSlate-400 truncate text-xs">
                      {instance.gameVersion} • {instance.modloader}
                    </div>
                  </div>
                </div>
              )}
            </VList>
          </div>
        </Show>
      </PopoverContent>
    </Popover>
  )
}
