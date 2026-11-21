import { useLocation } from "@solidjs/router"
import { useTransContext } from "@gd/i18n"
import { withStates } from "@gd/ui"
import type { AccountEntry, AccountStatus } from "@gd/core_module/bindings"
import {
  For,
  JSX,
  Match,
  Show,
  Switch,
  createEffect,
  createSignal,
  onCleanup
} from "solid-js"
import { useGDNavigate } from "@/managers/NavigationManager"
import { apiUrl, rspc } from "@/utils/rspcClient"
import { logoUrl } from "@/utils/logos"
import { getAccountImageUuid } from "@/utils/showcaseHelpers"
import { useGlobalStore } from "../GlobalStoreContext"
import { needsAttention, stripAccounts } from "./model"
import { addAccountPath, animatesOpen, watchOpenStrip } from "./behaviour"
import { motion } from "@/themes/motion"

const SETTINGS = "/settings/accounts"

const headUrl = (account: AccountEntry) =>
  apiUrl(`/account/headImage?uuid=${getAccountImageUuid(account)}`)

const StatusIcon = (props: { status: AccountStatus }) => (
  <Switch>
    <Match when={props.status === "ok"}>
      <div class="i-hugeicons:tick-02 h-4 w-4 shrink-0 text-green-500" />
    </Match>
    <Match when={props.status === "expired"}>
      <div class="i-hugeicons:alert-01 h-4 w-4 shrink-0 text-yellow-500" />
    </Match>
    <Match when={props.status === "refreshing"}>
      <div class="i-hugeicons:loading-03 h-4 w-4 shrink-0 text-yellow-500" />
    </Match>
    <Match when={props.status === "invalid"}>
      <div class="i-hugeicons:cancel-01 h-4 w-4 shrink-0 text-red-500" />
    </Match>
  </Switch>
)

// The buttons in the strip. No outline utility: the focus ring comes from
// themes/controls.css, which a theme can restyle. Colours, sizes and gaps are
// per kind, never two of a kind on one button: UnoCSS orders utilities by its
// own sort, not by class order, so the later class wouldn't win.
const BASE =
  "flex h-7 shrink-0 cursor-pointer items-center rounded-md border-0 px-1.5 transition-colors hover:bg-darkSlate-600 hover:text-lightSlate-50"
const ITEM = `${BASE} gap-2 bg-transparent text-sm text-lightSlate-500`
const SMALL_ITEM = `${BASE} gap-2 bg-transparent text-xs text-lightSlate-500`
// Its lighter fill is in themes/controls.css: the reset clears a button's
// background utility.
const ACTIVE = `${BASE} gap-2 text-sm text-lightSlate-50`
const FACE = `${BASE} group relative bg-transparent text-sm text-lightSlate-500`

/** The top bar's account button. It opens a strip inside the top bar, never
 *  below it: the ad column under the bar is a native view no menu can draw
 *  over. The strip shows the active account by name and a few others as
 *  faces, whose names slide out on hover or focus. */
export const AccountStrip = (props: {
  /** The 48 px compact top bar. */
  dense?: boolean
  /** Told when the strip opens or closes, so the bar can make room. */
  onOpenChange?: (open: boolean) => void
}) => {
  const globalStore = useGlobalStore()
  const navigator = useGDNavigate()
  const location = useLocation()
  const [t] = useTransContext()
  const [open, setOpen] = createSignal(false)
  let root: HTMLDivElement | undefined
  let button: HTMLButtonElement | undefined
  let strip: HTMLDivElement | undefined

  const setActiveAccount = rspc.createMutation(() => ({
    mutationKey: ["account.setActiveUuid"]
  }))

  const model = () =>
    stripAccounts(
      globalStore.accounts.data ?? [],
      globalStore.currentlySelectedAccountUuid.data
    )
  const validGDLUser = () =>
    globalStore.gdlAccount.data?.status === "valid"
      ? globalStore.gdlAccount.data?.value
      : undefined
  const gdlAccountUnavailable = () =>
    globalStore.gdlAccount.isError && !!globalStore.settings.data?.gdlAccountId

  createEffect(() => props.onOpenChange?.(open()))
  // Gone while open (the accounts query reset): the bar shows its buttons.
  onCleanup(() => props.onOpenChange?.(false))

  const close = (returnFocus: boolean) => {
    setOpen(false)
    if (returnFocus)
      queueMicrotask(() => button?.focus({ preventScroll: true }))
  }
  const go = (path: string) => {
    close(false)
    navigator.navigate(path)
  }
  const switchTo = (uuid: string) => {
    setActiveAccount.mutate(uuid)
    close(true)
  }

  const items = () =>
    strip ? [...strip.querySelectorAll<HTMLButtonElement>("button")] : []

  // Opened with the keyboard, focus goes to the active account (or the first
  // button); opened with a pointer, to the strip itself, so no ring shows.
  const openStrip = (byKeyboard: boolean) => {
    setOpen(true)
    queueMicrotask(() => {
      if (!strip) return
      if (animatesOpen(motion())) {
        strip.animate(
          [
            { opacity: 0, transform: "scaleX(0.9)" },
            { opacity: 1, transform: "none" }
          ],
          { duration: 150, easing: "cubic-bezier(.4,0,.2,1)" }
        )
      }
      if (!byKeyboard) {
        strip.focus({ preventScroll: true })
        return
      }
      const active = strip.querySelector<HTMLButtonElement>(
        '[data-gdl~="account-menu.account.active"]'
      )
      ;(active ?? items()[0])?.focus({ preventScroll: true })
    })
  }

  const onStripKeyDown: JSX.EventHandler<HTMLDivElement, KeyboardEvent> = (
    e
  ) => {
    const list = items()
    if (list.length === 0) return
    const at = list.indexOf(document.activeElement as HTMLButtonElement)
    let next: number | null = null
    if (e.key === "ArrowRight") next = at < 0 ? 0 : (at + 1) % list.length
    else if (e.key === "ArrowLeft")
      next = at < 0 ? list.length - 1 : (at - 1 + list.length) % list.length
    else if (e.key === "Home") next = 0
    else if (e.key === "End") next = list.length - 1
    if (next === null) return
    e.preventDefault()
    list[next].focus()
  }

  // While open: Esc in it, focus leaving it, or a press elsewhere closes it
  // (behaviour.ts).
  createEffect(() => {
    if (!open() || !root) return
    onCleanup(watchOpenStrip(root, close))
  })

  return (
    <div ref={root} class="relative flex items-center">
      <button
        ref={button}
        type="button"
        data-gdl="account-menu.button"
        aria-expanded={open()}
        class="cursor-pointer border-0 bg-transparent p-0 text-inherit"
        style={{ display: open() ? "none" : undefined }}
        onClick={(e) => openStrip(e.detail === 0)}
      >
        <div
          data-gdl={withStates("account-menu.trigger", { open: open() })}
          class="group relative rounded-lg transition-all duration-100 ease-spring"
          classList={{ "p-4": !props.dense, "px-4 py-2": props.dense }}
        >
          <div
            data-gdl="account-menu.highlight"
            class="bg-darkSlate-700 -z-1 absolute left-0 top-0 h-full w-full scale-75 rounded-md opacity-0 transition-[transform,opacity] duration-150 ease-[cubic-bezier(.4,0,.2,1)] group-hover:scale-100 group-hover:opacity-100"
          />
          <div class="flex items-center gap-4">
            <img
              src={(() => {
                const active = model().active
                return active
                  ? headUrl(active)
                  : apiUrl("/account/headImage?uuid=")
              })()}
              alt=""
              class="h-6 w-6 rounded-md"
            />
            <div class="max-w-30 truncate">{model().active?.username}</div>
            <div class="i-hugeicons:arrow-down-01 h-4 w-4" />
          </div>
        </div>
      </button>
      <Show when={open()}>
        <div
          ref={strip}
          data-gdl="account-menu.strip"
          role="toolbar"
          aria-label={t("accounts:_trn_minecraft_accounts")}
          tabIndex={-1}
          class="bg-darkSlate-700 flex shrink-0 origin-right items-center gap-1 rounded-lg px-1.5 outline-none"
          style={{ height: `${props.dense ? 36 : 40}px` }}
          onKeyDown={onStripKeyDown}
        >
          <Show when={model().active}>
            {(active) => (
              <button
                type="button"
                data-gdl={withStates("account-menu.account", {
                  active: true,
                  attention: !!needsAttention(active().status)
                })}
                class={ACTIVE}
                onClick={() => close(true)}
              >
                <img
                  src={headUrl(active())}
                  alt=""
                  class="h-6 w-6 rounded-md"
                />
                <span
                  data-gdl="account-menu.account.name"
                  class="max-w-30 truncate"
                >
                  {active().username}
                </span>
                <StatusIcon status={active().status} />
              </button>
            )}
          </Show>
          <For each={model().faces}>
            {(account) => {
              const attention = () => needsAttention(account.status)
              return (
                <button
                  type="button"
                  aria-label={account.username}
                  data-gdl={withStates("account-menu.account", {
                    attention: !!attention()
                  })}
                  class={FACE}
                  onClick={() => switchTo(account.uuid)}
                >
                  <span class="relative shrink-0">
                    <img
                      src={headUrl(account)}
                      alt=""
                      class="block h-6 w-6 rounded-md"
                    />
                    <Show when={attention()}>
                      <span
                        class="absolute -right-0.5 -top-0.5 h-1.5 w-1.5 rounded-full"
                        classList={{
                          "bg-yellow-500": attention() === "warning",
                          "bg-red-500": attention() === "error"
                        }}
                      />
                    </Show>
                  </span>
                  {/* Over the bar, beside the face, never part of the
                      layout: a name that pushed the faces aside would move
                      them out from under the pointer. */}
                  <span
                    data-gdl="account-menu.account.name"
                    class="bg-darkSlate-600 text-lightSlate-50 pointer-events-none invisible absolute right-full top-0 z-10 mr-1 flex h-7 items-center whitespace-nowrap rounded-md px-2 opacity-0 transition-opacity duration-150 group-hover:visible group-hover:opacity-100 group-focus-visible:visible group-focus-visible:opacity-100 motion-reduce:transition-none"
                  >
                    <span class="max-w-30 truncate">{account.username}</span>
                  </span>
                </button>
              )
            }}
          </For>
          <Show when={model().more > 0}>
            <button
              type="button"
              data-gdl="account-menu.more"
              class={ITEM}
              onClick={() => go(SETTINGS)}
            >
              +{model().more}
            </button>
          </Show>
          <button
            type="button"
            data-gdl="account-menu.add"
            aria-label={t("accounts:_trn_add_account")}
            class={ITEM}
            onClick={() => go(addAccountPath(location))}
          >
            <div class="i-hugeicons:add-01 h-5 w-5" />
          </button>
          <div class="bg-darkSlate-500 mx-1 h-5 w-px shrink-0" />
          <button
            type="button"
            data-gdl="account-menu.gdl"
            class={SMALL_ITEM}
            onClick={() => go(SETTINGS)}
          >
            <img src={logoUrl} alt="" class="h-5 w-5" />
            <Switch
              fallback={
                <span class="max-w-30 truncate">
                  {t("accounts:_trn_no_account_synced")}
                </span>
              }
            >
              <Match when={validGDLUser()}>
                {(user) => (
                  <span class="max-w-30 truncate">{user().displayName}</span>
                )}
              </Match>
              <Match when={gdlAccountUnavailable()}>
                <span class="max-w-30 truncate text-yellow-400">
                  {t("accounts:_trn_gdl_account_unavailable")}
                </span>
              </Match>
            </Switch>
          </button>
          <button
            type="button"
            data-gdl="account-menu.manage"
            aria-label={t("accounts:_trn_manage_accounts")}
            class={ITEM}
            onClick={() => go(SETTINGS)}
          >
            <div class="i-hugeicons:settings-01 h-5 w-5" />
          </button>
          <button
            type="button"
            data-gdl="account-menu.close"
            aria-label={t("general:_trn_close")}
            class={ITEM}
            onClick={() => close(true)}
          >
            <div class="i-hugeicons:arrow-up-01 h-4 w-4" />
          </button>
        </div>
      </Show>
    </div>
  )
}

export default AccountStrip
