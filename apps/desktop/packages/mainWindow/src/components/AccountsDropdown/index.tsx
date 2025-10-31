import { useGDNavigate } from "@/managers/NavigationManager"

import { port, rspc } from "@/utils/rspcClient"
import { AccountStatus, AccountType } from "@gd/core_module/bindings"
import { Trans } from "@gd/i18n"
import { Button, Popover, PopoverContent, PopoverTrigger } from "@gd/ui"
import { For, Switch, Match, createSignal } from "solid-js"
import gdlLogo from "/assets/images/gdlauncher_logo.svg"
import defaultInstanceImg from "/assets/images/default-instance-img.png"
import { useGlobalStore } from "../GlobalStoreContext"
import { getAccountImageUuid } from "@/utils/showcaseHelpers"

export interface Label {
  name: string
  icon: string | undefined
  uuid: string
  type: AccountType
  status: AccountStatus | undefined
}

export interface Account {
  label: Label
  key: string
}

export interface OptionDropDown {
  label: string
  key: string
}

export interface Props {
  accounts: Account[]
  value: string | null | undefined
  disabled?: boolean
  label?: string
  id?: string
}

export const AccountsDropdown = (props: Props) => {
  const globalStore = useGlobalStore()
  const navigator = useGDNavigate()
  const [showAccountsDropdown, setShowAccountsDropdown] = createSignal(false)

  const setActiveAccountMutation = rspc.createMutation(() => ({
    mutationKey: ["account.setActiveUuid"]
  }))

  const validGDLUser = () =>
    globalStore.gdlAccount.data?.status === "valid"
      ? globalStore.gdlAccount.data?.value
      : undefined

  let gdlAccountRef: HTMLDivElement | undefined
  let mcAccountsRef: HTMLDivElement | undefined
  let settingsButtonRef: HTMLDivElement | undefined

  return (
    <Popover
      placement="bottom"
      open={showAccountsDropdown()}
      onOpenChange={(open) => {
        setShowAccountsDropdown(open)

        if (!open) return

        if (gdlAccountRef) {
          gdlAccountRef.animate(
            [
              {
                opacity: 0
              },
              {
                opacity: 1
              }
            ],
            {
              duration: 500,
              easing: "cubic-bezier(0.175, 0.885, 0.32, 1.275)",
              fill: "forwards"
            }
          )
        }

        if (mcAccountsRef) {
          mcAccountsRef.animate(
            [
              {
                opacity: 0
              },
              {
                opacity: 1
              }
            ],
            {
              duration: 500,
              easing: "cubic-bezier(0.175, 0.885, 0.32, 1.275)",
              fill: "forwards",
              delay: 50
            }
          )
        }

        if (settingsButtonRef) {
          settingsButtonRef.animate(
            [
              {
                opacity: 0
              },
              {
                opacity: 100
              }
            ],
            {
              duration: 500,
              easing: "cubic-bezier(0.175, 0.885, 0.32, 1.275)",
              fill: "forwards",
              delay: 100
            }
          )
        }
      }}
    >
      <PopoverTrigger>
        <div
          class="group relative rounded-lg p-4 transition-all duration-100 ease-in-out"
          classList={{
            "bg-darkSlate-700": showAccountsDropdown()
          }}
        >
          <div class="bg-darkSlate-700 -z-1 absolute left-0 top-0 h-full w-full scale-75 rounded-md opacity-0 transition-[transform,opacity] duration-150 ease-[cubic-bezier(.4,0,.2,1)] group-hover:scale-100 group-hover:opacity-100" />

          <div class="flex items-center gap-4">
            <img
              src={`http://127.0.0.1:${port}/account/headImage?uuid=${(() => {
                const account = globalStore.accounts.data?.find(
                  (account) =>
                    account.uuid ===
                    globalStore.currentlySelectedAccountUuid.data
                )
                return account ? getAccountImageUuid(account) : ""
              })()}`}
              class="h-6 w-6 rounded-md"
            />
            <div class="max-w-30 truncate">
              {
                globalStore.accounts.data?.find(
                  (account) =>
                    account.uuid ===
                    globalStore.currentlySelectedAccountUuid.data
                )?.username
              }
            </div>
            <div class="i-hugeicons:arrow-down-01 h-4 w-4" />
          </div>
        </div>
      </PopoverTrigger>
      <PopoverContent
        class="w-80 border-none bg-transparent p-0"
        hideCloseButton
      >
        <div class="flex flex-col gap-1">
          <div
            class="bg-darkSlate-700 shadow-darkSlate-900 mr-2 h-auto w-full rounded-lg p-2 opacity-0 shadow-lg transition-opacity"
            ref={gdlAccountRef}
          >
            <div class="flex items-center gap-4 px-4 py-2 text-xl">
              <img src={gdlLogo} class="h-6 w-6" />
              <div>
                <Trans key="GDLauncher_account" />
              </div>
            </div>
            <hr class="border-darkSlate-50 w-full opacity-20" />
            <Switch
              fallback={
                <div class="flex items-center gap-4 rounded-lg px-4 py-2">
                  <div class="bg-darkSlate-600 h-6 w-6 rounded-md" />
                  <div>
                    <Trans key="No_account_synced" />
                  </div>
                </div>
              }
            >
              <Match when={validGDLUser()}>
                <div class="flex items-center gap-4 rounded-lg px-4 py-2">
                  <img
                    src={validGDLUser()?.profileIconUrl}
                    class="h-6 w-6 rounded-md"
                  />
                  <div class="max-w-50 truncate">
                    {validGDLUser()?.nickname}
                  </div>
                </div>
              </Match>
            </Switch>
          </div>
          <div
            class="bg-darkSlate-700 shadow-darkSlate-900 mr-2 h-auto w-full rounded-lg p-2 opacity-0 shadow-lg transition-opacity"
            ref={mcAccountsRef}
          >
            <div class="flex items-center gap-4 px-4 py-2 text-xl">
              <img src={defaultInstanceImg} class="h-6 w-6" />
              <div>
                <Trans key="Minecraft_accounts" />
              </div>
            </div>
            <hr class="border-darkSlate-50 w-full opacity-20" />
            <For each={globalStore.accounts.data || []}>
              {(account) => (
                <div
                  class="hover:bg-darkSlate-600 flex items-center justify-between gap-4 rounded-lg px-4 py-2"
                  classList={{
                    "bg-darkSlate-600":
                      account.uuid ===
                      globalStore.currentlySelectedAccountUuid.data
                  }}
                  onClick={() => {
                    setActiveAccountMutation.mutate(account.uuid)
                  }}
                >
                  <div class="flex items-center gap-4">
                    <img
                      src={`http://127.0.0.1:${port}/account/headImage?uuid=${getAccountImageUuid(account)}`}
                      class="h-6 w-6 rounded-md"
                    />
                    <div class="max-w-30 truncate">{account.username}</div>
                  </div>
                  <div class="flex items-center gap-2">
                    <Switch>
                      <Match when={account.type.type === "microsoft"}>
                        <div class="i-hugeicons:microsoft h-4 w-4" />
                      </Match>
                      <Match when={account.type.type === "offline"}>
                        <div class="i-hugeicons:computer h-4 w-4" />
                      </Match>
                    </Switch>
                    <Switch>
                      <Match when={account.status === "ok"}>
                        <div class="i-hugeicons:tick-02 h-4 w-4 text-green-500" />
                      </Match>
                      <Match when={account.status === "expired"}>
                        <div class="i-hugeicons:alert-01 h-4 w-4 text-yellow-500" />
                      </Match>
                      <Match when={account.status === "refreshing"}>
                        <div class="i-hugeicons:loading-03 h-4 w-4 text-yellow-500" />
                      </Match>
                      <Match when={account.status === "invalid"}>
                        <div class="i-hugeicons:cancel-01 h-4 w-4 text-red-500" />
                      </Match>
                    </Switch>
                  </div>
                </div>
              )}
            </For>
          </div>
          <div
            class="bg-darkSlate-700 shadow-darkSlate-900 mr-2 h-auto w-full rounded-lg p-2 opacity-0 shadow-lg transition-opacity"
            ref={settingsButtonRef}
          >
            <Button
              type="outline"
              class="flex items-center justify-center gap-4"
              fullWidth
              onClick={() => {
                if (props.disabled) return
                setShowAccountsDropdown(false)
                navigator.navigate("/settings/accounts")
              }}
            >
              <div
                class="i-hugeicons:settings-01 text-2xl pointer-events-auto"
                classList={{
                  "text-lightSlate-50": !!props.disabled,
                  "hover:text-lightSlate-100 duration-100 ease-in-out":
                    !!props.disabled
                }}
              />
              <div>
                <Trans key="settings:manage_accounts" />
              </div>
            </Button>
          </div>
        </div>
      </PopoverContent>
    </Popover>
  )
}
