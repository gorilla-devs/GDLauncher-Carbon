import {
  flexRender,
  getCoreRowModel,
  ColumnDef,
  createSolidTable
} from "@tanstack/solid-table"
import { Trans, useTransContext } from "@gd/i18n"
import {
  Button,
  toast,
  Popover,
  PopoverContent,
  PopoverTrigger,
  Tooltip,
  TooltipContent,
  TooltipTrigger,
  Collapsable
} from "@gd/ui"
import { port, queryClient, rspc } from "@/utils/rspcClient"
import PageTitle from "./components/PageTitle"
import Row from "./components/Row"
import Title from "./components/Title"
import RightHandSide from "./components/RightHandSide"
import RowsContainer from "./components/RowsContainer"
import CopyableField from "./components/CopyableField"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import {
  createEffect,
  createMemo,
  createSignal,
  For,
  Match,
  Show,
  Switch
} from "solid-js"
import { useGDNavigate } from "@/managers/NavigationManager"
import { convertSecondsToHumanTime } from "@/utils/helpers"
import { useModal } from "@/managers/ModalsManager"
import { AccountEntry } from "@gd/core_module/bindings"
import { getAccountImageUuid } from "@/utils/showcaseHelpers"

const defaultColumns: ColumnDef<AccountEntry>[] = [
  {
    accessorFn: () => <></>,
    id: "active",
    cell: (info) => info.getValue(),
    header: () => (
      <span>
        <Trans key="accounts:_trn_active" />
      </span>
    )
  },
  {
    accessorFn: (row) => row.username,
    id: "username",
    cell: (info) => (
      <div class="flex items-center gap-4">
        <img
          src={`http://127.0.0.1:${port}/account/headImage?uuid=${getAccountImageUuid(info.row.original)}`}
          class="h-8 w-8 rounded-md"
        />
        <div class="max-w-50 2xl:max-w-100 truncate">
          {info.row.original.username}
        </div>
      </div>
    ),
    header: () => (
      <span>
        <Trans key="accounts:_trn_username" />
      </span>
    )
  },
  {
    accessorFn: (row) => row.type.type,
    id: "type",
    cell: (info) => info.getValue(),
    header: () => (
      <span>
        <Trans key="accounts:_trn_type" />
      </span>
    )
  },
  {
    accessorFn: (row) => row.status,
    id: "status",
    cell: (info) => (
      <div class="flex items-center justify-center">
        <Switch>
          <Match when={info.getValue() === "ok"}>
            <div class="i-hugeicons:tick-02 text-green-500 h-4 w-4" />
          </Match>
          <Match when={info.getValue() === "expired"}>
            <div class="i-hugeicons:alert-01 text-yellow-500 h-4 w-4" />
          </Match>
          <Match when={info.getValue() === "refreshing"}>
            <div class="i-hugeicons:refresh text-yellow-500 h-4 w-4" />
          </Match>
          <Match when={info.getValue() === "invalid"}>
            <div class="i-hugeicons:cancel-01 text-red-500 h-4 w-4" />
          </Match>
        </Switch>
      </div>
    ),
    header: () => (
      <span>
        <Trans key="accounts:_trn_status" />
      </span>
    )
  },
  {
    accessorFn: (row) => row.uuid,
    id: "uuid",
    cell: (info) => (
      <div>
        <div class="max-w-50 2xl:max-w-100 truncate">
          {info.getValue() as string}
        </div>
      </div>
    ),
    header: () => (
      <span>
        <Trans key="accounts:_trn_uuid" />
      </span>
    )
  },
  {
    accessorFn: () => <></>,
    id: "actions",
    cell: (info) => info.getValue(),
    header: () => (
      <span>
        <Trans key="accounts:_trn_actions" />
      </span>
    )
  }
]

const Accounts = () => {
  const globalStore = useGlobalStore()
  const [t] = useTransContext()

  const gdNavigator = useGDNavigate()
  const modalsContext = useModal()

  const removeGDLAccountMutation = rspc.createMutation(() => ({
    mutationKey: ["account.removeGdlAccount"]
  }))

  const removeAccountMutation = rspc.createMutation(() => ({
    mutationKey: ["account.deleteAccount"]
  }))

  const setActiveAccountMutation = rspc.createMutation(() => ({
    mutationKey: ["account.setActiveUuid"]
  }))

  // Avatar preview state
  const [avatarPreview, setAvatarPreview] = createSignal<string | null>(null)

  const clearDisplayNameHistoryMutation = rspc.createMutation(() => ({
    mutationKey: ["account.clearDisplayNameHistory"]
  }))

  const validGDLUser = () =>
    globalStore.gdlAccount.data?.status === "valid"
      ? globalStore.gdlAccount.data?.value
      : undefined

  const friendCode = createMemo(() => validGDLUser()?.friendCode)

  const displayNameHistoryQuery = rspc.createQuery(() => ({
    queryKey: ["account.getDisplayNameHistory", friendCode() ?? ""],
    enabled: !!friendCode()
  }))

  const invalidGDLUser = () => globalStore.gdlAccount.data?.status === "invalid"

  // Initialize avatar preview from GDL account
  createEffect(() => {
    const url = validGDLUser()?.profileIconUrl
    if (url) {
      setAvatarPreview(url)
    } else {
      setAvatarPreview(null)
    }
  })

  // Helper to calculate remaining seconds from an absolute UTC timestamp
  const getRemainingSeconds = (
    timeoutAt: string | null | undefined,
    fallbackSeconds: number | null | undefined
  ): number => {
    if (timeoutAt) {
      const expiresAt = new Date(timeoutAt).getTime()
      const remaining = Math.floor((expiresAt - Date.now()) / 1000)
      return Math.max(0, remaining)
    }
    return fallbackSeconds && fallbackSeconds > 0 ? fallbackSeconds : 0
  }

  const deletionCooldownRemaining = () => {
    const user = validGDLUser()
    return getRemainingSeconds(user?.deletionTimeoutAt, user?.deletionTimeout)
  }

  const deleteAccountContent = () => {
    const remaining = deletionCooldownRemaining()
    if (remaining > 0) {
      return (
        <Trans
          key="accounts:_trn_cannot_request_deletion_for_time"
          options={{
            time: convertSecondsToHumanTime(remaining)
          }}
        />
      )
    } else {
      return undefined
    }
  }

  const accountsTable = createSolidTable({
    get data() {
      return globalStore.accounts.data || []
    },
    columns: defaultColumns,
    getCoreRowModel: getCoreRowModel()
  })

  return (
    <>
      <PageTitle>
        <Trans key="accounts:_trn_accounts" />
      </PageTitle>
      {/* GDL Account Section */}
      <Switch>
        {/* Logged in state - Discord-like profile card */}
        <Match when={validGDLUser()}>
          <div class="bg-darkSlate-700 rounded-xl p-6">
            {/* Profile Header */}
            <div class="mb-4 flex items-center justify-between">
              <h2 class="text-lightSlate-50 text-lg font-semibold">
                <Trans key="accounts:_trn_gdl_account_title" />
              </h2>
              <div class="flex items-center gap-2">
                <Button
                  type="secondary"
                  size="small"
                  onClick={() =>
                    modalsContext?.openModal({ name: "editGDLProfile" })
                  }
                >
                  <div class="i-hugeicons:edit-02" />
                  <Trans key="accounts:_trn_edit_profile" />
                </Button>
                <Popover>
                  <PopoverTrigger>
                    <Button type="secondary" size="small">
                      <div class="i-hugeicons:logout-01" />
                      <Trans key="accounts:_trn_log_out" />
                    </Button>
                  </PopoverTrigger>
                  <PopoverContent class="w-80" hideCloseButton>
                    <div class="flex flex-col gap-3">
                      <p class="text-lightSlate-200 m-0 text-sm">
                        <Trans key="accounts:_trn_log_out_description" />
                      </p>
                      <div class="flex justify-end">
                        <Button
                          type="secondary"
                          size="small"
                          onClick={() => {
                            removeGDLAccountMutation.mutate(undefined)
                          }}
                        >
                          <div class="i-hugeicons:logout-01" />
                          <Trans key="accounts:_trn_log_out" />
                        </Button>
                      </div>
                    </div>
                  </PopoverContent>
                </Popover>
              </div>
            </div>

            {/* Profile Card */}
            <div class="flex items-start gap-6">
              {/* Avatar - Display only */}
              <img
                src={
                  avatarPreview() ||
                  `http://127.0.0.1:${port}/account/headImage?uuid=${globalStore.settings.data?.gdlAccountId}`
                }
                class="h-20 w-20 rounded-xl"
              />

              {/* Profile Info */}
              <div class="flex-1">
                {/* Display Name with History */}
                <div class="mb-3">
                  <span class="text-lightSlate-500 text-sm">
                    <Trans key="accounts:_trn_display_name" />
                  </span>
                  <div class="flex items-center gap-1">
                    <span class="text-lightSlate-50 text-xl font-semibold">
                      {validGDLUser()?.displayName}
                    </span>
                    <Show
                      when={
                        displayNameHistoryQuery.data &&
                        displayNameHistoryQuery.data.length > 0
                      }
                    >
                      <Popover>
                        <PopoverTrigger>
                          <button class="text-lightSlate-500 hover:text-lightSlate-300 flex items-center justify-center rounded p-1 transition-colors hover:bg-darkSlate-600">
                            <div class="i-hugeicons:arrow-down-01 text-lg" />
                          </button>
                        </PopoverTrigger>
                        <PopoverContent class="max-h-60 w-64 overflow-y-auto">
                          <div class="flex flex-col gap-2">
                            <div class="text-lightSlate-50 font-medium">
                              <Trans key="accounts:_trn_previous_names" />
                            </div>
                            <For each={displayNameHistoryQuery.data}>
                              {(entry) => (
                                <div class="text-lightSlate-300 flex justify-between text-sm">
                                  <span>{entry.displayName}</span>
                                  <span class="text-lightSlate-500">
                                    {new Date(
                                      entry.changedAt
                                    ).toLocaleDateString()}
                                  </span>
                                </div>
                              )}
                            </For>
                            <Button
                              type="secondary"
                              size="small"
                              class="mt-2"
                              onClick={async () => {
                                const uuid =
                                  globalStore?.currentlySelectedAccountUuid
                                    ?.data
                                if (!uuid) return

                                try {
                                  await clearDisplayNameHistoryMutation.mutateAsync(
                                    uuid
                                  )
                                  queryClient.invalidateQueries({
                                    queryKey: ["account.getDisplayNameHistory"]
                                  })
                                  toast.success(
                                    t(
                                      "accounts:_trn_display_name_history_cleared"
                                    )
                                  )
                                } catch (err) {
                                  console.error(err)
                                  toast.error(
                                    t(
                                      "accounts:_trn_display_name_history_clear_failed"
                                    )
                                  )
                                }
                              }}
                            >
                              <Trans key="accounts:_trn_clear_display_name_history" />
                            </Button>
                          </div>
                        </PopoverContent>
                      </Popover>
                    </Show>
                  </div>
                </div>

                {/* Info Fields */}
                <div class="space-y-1">
                  <CopyableField
                    label={t("accounts:_trn_friend_code")}
                    value={validGDLUser()?.friendCode}
                  />
                  <div class="text-lightSlate-300 flex items-center gap-2 text-sm">
                    <span class="text-lightSlate-500">
                      <Trans key="accounts:_trn_microsoft_account" />:
                    </span>
                    <img
                      src={`http://127.0.0.1:${port}/account/headImage?uuid=${globalStore.settings.data?.gdlAccountId}`}
                      class="h-4 w-4 rounded"
                    />
                    <span>
                      {
                        globalStore.accounts.data?.find(
                          (account) =>
                            account.uuid ===
                            globalStore.settings.data?.gdlAccountId
                        )?.username
                      }
                    </span>
                  </div>
                  <div class="text-lightSlate-300 flex items-center gap-2 text-sm">
                    <span class="text-lightSlate-500">
                      <Trans key="accounts:_trn_recovery_email" />:
                    </span>
                    <span>{validGDLUser()?.email || "-"}</span>
                  </div>
                  <div class="text-lightSlate-300 flex items-center gap-2 text-sm">
                    <span class="text-lightSlate-500">
                      <Trans key="accounts:_trn_status" />:
                    </span>
                    <Show
                      when={validGDLUser()?.isEmailVerified}
                      fallback={
                        <span class="text-yellow-400">
                          <Trans key="accounts:_trn_not_verified" />
                        </span>
                      }
                    >
                      <span class="flex items-center gap-1 text-green-400">
                        <div class="i-hugeicons:tick-02" />
                        <Trans key="accounts:_trn_verified" />
                      </span>
                    </Show>
                  </div>
                </div>
              </div>
            </div>

            {/* Actions */}
            <div class="border-darkSlate-500 mt-6 flex gap-3 border-t pt-4">
              <Button
                type="secondary"
                size="small"
                onClick={() => {
                  modalsContext?.openModal({ name: "myShares" })
                }}
              >
                <div class="i-ri:share-line" />
                <Trans key="accounts:_trn_view_shares" />
              </Button>
            </div>

            {/* Danger Zone - Collapsible */}
            <div class="border-darkSlate-500 mt-4 border-t pt-2">
              <Collapsable
                title={
                  <span class="text-red-400 normal-case">
                    <Trans key="accounts:_trn_danger_zone" />
                  </span>
                }
                defaultOpened={false}
                noPadding
                size="small"
              >
                <div class="flex items-center justify-between py-2">
                  <p class="text-lightSlate-500 text-sm">
                    <Trans key="accounts:_trn_delete_account_description" />
                  </p>
                  <Show
                    when={deletionCooldownRemaining() > 0}
                    fallback={
                      <Button
                        type="secondary"
                        size="small"
                        class="text-red-400 hover:text-red-300"
                        onClick={() => {
                          modalsContext?.openModal({
                            name: "confirmGDLAccountDeletion"
                          })
                        }}
                      >
                        <Trans key="accounts:_trn_request_account_deletion" />
                      </Button>
                    }
                  >
                    <Tooltip>
                      <TooltipTrigger>
                        <Button
                          type="secondary"
                          size="small"
                          class="text-red-400 hover:text-red-300"
                          disabled
                        >
                          <Trans key="accounts:_trn_request_account_deletion" />
                        </Button>
                      </TooltipTrigger>
                      <TooltipContent>{deleteAccountContent()}</TooltipContent>
                    </Tooltip>
                  </Show>
                </div>
              </Collapsable>
            </div>
          </div>
        </Match>

        {/* Not logged in state */}
        <Match when={!validGDLUser() && !invalidGDLUser()}>
          <RowsContainer>
            <Row>
              <Title>
                <Trans key="accounts:_trn_gdl_account_title" />
              </Title>
            </Row>
            <Row>
              <Title>
                <span class="text-red-400">
                  <Trans key="accounts:_trn_gdl_account_not_synced" />
                </span>
              </Title>
              <RightHandSide>
                <Button
                  type="secondary"
                  onClick={async () => {
                    await removeGDLAccountMutation.mutateAsync(undefined)
                    gdNavigator.navigate(
                      "/?addGdlAccount=true&returnTo=/settings/accounts"
                    )
                  }}
                >
                  <div class="i-hugeicons:link-01" />
                  <Trans key="accounts:_trn_link_gdl_account" />
                </Button>
              </RightHandSide>
            </Row>
          </RowsContainer>
        </Match>

        {/* Error state */}
        <Match when={invalidGDLUser()}>
          <RowsContainer>
            <Row>
              <Title>
                <Trans key="accounts:_trn_gdl_account_title" />
              </Title>
            </Row>
            <Row>
              <Title>
                <span class="text-yellow-400">
                  <Trans key="accounts:_trn_gdl_account_error" />
                </span>
              </Title>
              <RightHandSide>
                <Button
                  type="secondary"
                  onClick={() => {
                    removeGDLAccountMutation.mutate(undefined)
                  }}
                >
                  <div class="i-hugeicons:logout-01" />
                  <Trans key="accounts:_trn_log_out_gdl_account" />
                </Button>
              </RightHandSide>
            </Row>
          </RowsContainer>
        </Match>
      </Switch>
      <RowsContainer>
        <Row forceContentBelow>
          <Title>
            <div class="flex items-center gap-4">
              <Trans key="accounts:_trn_minecraft_accounts" />
              <Button
                type="secondary"
                size="small"
                onClick={() => {
                  gdNavigator.navigate(
                    "/?addMicrosoftAccount=true&returnTo=/settings/accounts"
                  )
                }}
              >
                <div class="i-hugeicons:add-01 text-lg" />
              </Button>
            </div>
          </Title>
        </Row>

        <table class="table-auto border-collapse">
          <thead>
            <For each={accountsTable.getHeaderGroups()}>
              {(headerGroup) => (
                <tr>
                  <For each={headerGroup.headers}>
                    {(header, i) => (
                      <th
                        class={`text-lightSlate-900 border-darkSlate-500 border-0 border-solid ${i() !== 0 ? "border-l-1" : ""}`}
                      >
                        {header.isPlaceholder
                          ? null
                          : flexRender(
                              header.column.columnDef.header,
                              header.getContext()
                            )}
                      </th>
                    )}
                  </For>
                </tr>
              )}
            </For>
          </thead>
          <tbody>
            <For each={accountsTable.getRowModel().rows}>
              {(row) => (
                <tr class="hover:bg-darkSlate-700 group/external transition-colors duration-100 ease-spring">
                  <For each={row.getVisibleCells()}>
                    {(cell, i) => (
                      <td
                        class="group/internal text-lightSlate-300 group-hover/external:border-darkSlate-500 relative border-0 border-solid border-transparent px-4 py-2"
                        classList={{
                          "hover:text-lightSlate-50":
                            cell.column.columnDef.id === "username" ||
                            cell.column.columnDef.id === "uuid",
                          "border-l-1": i() !== 0
                        }}
                        onClick={() => {
                          if (
                            cell.column.columnDef.id === "active" &&
                            row.original.uuid !==
                              globalStore.currentlySelectedAccountUuid.data
                          ) {
                            setActiveAccountMutation.mutate(row.original.uuid)
                          } else if (
                            cell.column.columnDef.id === "uuid" ||
                            cell.column.columnDef.id === "username"
                          ) {
                            navigator.clipboard.writeText(
                              cell.getValue() as string
                            )

                            toast.success("Copied to clipboard")
                          }
                        }}
                      >
                        <Switch>
                          <Match when={cell.column.columnDef.id === "actions"}>
                            <div class="flex w-full items-center justify-center gap-4">
                              <Show when={row.original.status !== "ok"}>
                                <div class="w-full text-yellow-500 hover:text-yellow-200">
                                  <div
                                    class="i-hugeicons:refresh h-4 w-4"
                                    onClick={async () => {
                                      gdNavigator.navigate(
                                        "/?addMicrosoftAccount=true&returnTo=/settings/accounts"
                                      )
                                    }}
                                  />
                                </div>
                              </Show>
                              <div class="flex w-full items-center justify-center hover:text-red-500">
                                <div
                                  class="i-hugeicons:delete-02 h-4 w-4"
                                  onClick={async () => {
                                    const gdlAccountUuid =
                                      globalStore.settings.data?.gdlAccountId
                                    const accountsLength =
                                      globalStore.accounts.data?.length

                                    if (
                                      gdlAccountUuid &&
                                      gdlAccountUuid === row.original.uuid
                                    ) {
                                      modalsContext?.openModal(
                                        {
                                          name: "confirmMsWithGDLAccountRemoval"
                                        },
                                        {
                                          uuid: row.original.uuid
                                        }
                                      )
                                    } else {
                                      await removeAccountMutation.mutateAsync(
                                        row.original.uuid
                                      )

                                      if (accountsLength === 1) {
                                        gdNavigator.navigate("/")
                                      }
                                    }
                                  }}
                                />
                              </div>
                            </div>
                          </Match>
                          <Match
                            when={
                              cell.column.columnDef.id === "active" &&
                              row.original.uuid ===
                                globalStore.currentlySelectedAccountUuid.data
                            }
                          >
                            <div class="flex items-center justify-center">
                              <div class="i-hugeicons:tick-double-02 text-lightSlate-50 h-4 w-4" />
                            </div>
                          </Match>
                          <Match
                            when={
                              cell.column.columnDef.id === "active" &&
                              row.original.uuid !==
                                globalStore.currentlySelectedAccountUuid.data
                            }
                          >
                            <div class="flex items-center justify-center opacity-0 duration-100 ease-spring group-hover/internal:opacity-100">
                              <div class="i-hugeicons:tick-double-02 text-darkSlate-300 h-4 w-4" />
                            </div>
                          </Match>
                          <Match
                            when={
                              cell.column.columnDef.id === "username" ||
                              cell.column.columnDef.id === "uuid"
                            }
                          >
                            <div class="absolute right-4 top-1/2 -translate-y-1/2 opacity-0 duration-100 ease-spring group-hover/internal:opacity-100">
                              <div class="i-hugeicons:clipboard text-lightSlate-50 text-lg" />
                            </div>
                          </Match>
                        </Switch>
                        {flexRender(
                          cell.column.columnDef.cell,
                          cell.getContext()
                        )}
                      </td>
                    )}
                  </For>
                </tr>
              )}
            </For>
          </tbody>
        </table>
      </RowsContainer>
    </>
  )
}

export default Accounts

// Handle automatic redirect to gdl login that fails on peek because
// all accounts are invalid because of the migration.
// Maybe show special login page for this case?
// or show gdl login but with all accounts disabled and a special message?
