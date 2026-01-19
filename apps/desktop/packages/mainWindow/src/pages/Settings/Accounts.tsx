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
  TooltipTrigger
} from "@gd/ui"
import { port, queryClient, rspc } from "@/utils/rspcClient"
import PageTitle from "./components/PageTitle"
import Row from "./components/Row"
import Title from "./components/Title"
import RowsContainer from "./components/RowsContainer"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import {
  createEffect,
  createMemo,
  createSignal,
  For,
  JSX,
  Match,
  Show,
  Switch
} from "solid-js"
import { useGDNavigate } from "@/managers/NavigationManager"
import { blobToBase64, convertSecondsToHumanTime } from "@/utils/helpers"
import ImagePicker from "@/components/ImagePicker"
import { useModal } from "@/managers/ModalsManager"
import { AccountEntry } from "@gd/core_module/bindings"
import { getAccountImageUuid } from "@/utils/showcaseHelpers"

const GDLAccountRowItem = (props: {
  title?: string
  value?: string | null | undefined
  children?: JSX.Element
  onEdit?: () => void
  extraAction?: JSX.Element
}) => {
  return (
    <div class="flex items-center justify-between">
      <div
        class="group flex flex-col justify-center gap-2"
        onClick={() => {
          if (!props.value) return

          navigator.clipboard.writeText(props.value)

          toast.success("Copied to clipboard")
        }}
      >
        <Show when={props.title}>
          <div class="text-lightSlate-700 group-hover:text-lightSlate-50 flex items-center gap-4 text-base font-light uppercase transition-all duration-100 ease-spring">
            {props.title}
            <Show when={props.onEdit}>
              <div
                class="text-md text-lightSlate-700 hover:text-lightSlate-50 underline transition-all duration-100 ease-spring"
                onClick={(e) => {
                  e.stopPropagation()
                  props.onEdit?.()
                }}
              >
                EDIT
              </div>
            </Show>
            {props.extraAction}
            <div class="hidden group-hover:block">
              <div class="i-hugeicons:clipboard text-lightSlate-50 text-lg" />
            </div>
          </div>
        </Show>
        <Show when={props.value}>
          <div class="text-lightSlate-50 overflow-hidden text-ellipsis whitespace-nowrap">
            {props.value}
          </div>
        </Show>
        {props.children}
      </div>
    </div>
  )
}

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

  const requestNewVerificationTokenMutation = rspc.createMutation(() => ({
    mutationKey: ["account.requestNewVerificationToken"]
  }))

  const removeAccountMutation = rspc.createMutation(() => ({
    mutationKey: ["account.deleteAccount"]
  }))

  const setActiveAccountMutation = rspc.createMutation(() => ({
    mutationKey: ["account.setActiveUuid"]
  }))

  // Avatar state and mutations
  const [avatarLoading, setAvatarLoading] = createSignal(false)
  const [avatarPreview, setAvatarPreview] = createSignal<string | null>(null)

  const uploadAvatarMutation = rspc.createMutation(() => ({
    mutationKey: ["account.uploadProfileIcon"]
  }))
  const deleteAvatarMutation = rspc.createMutation(() => ({
    mutationKey: ["account.deleteProfileIcon"]
  }))
  const clearNicknameHistoryMutation = rspc.createMutation(() => ({
    mutationKey: ["account.clearNicknameHistory"]
  }))

  const validGDLUser = () =>
    globalStore.gdlAccount.data?.status === "valid"
      ? globalStore.gdlAccount.data?.value
      : undefined

  const friendCode = createMemo(() => validGDLUser()?.friendCode)

  const nicknameHistoryQuery = rspc.createQuery(() => ({
    queryKey: ["account.getNicknameHistory", friendCode() ?? ""],
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

  const handleAvatarSelect = async (filePath: string) => {
    // Load preview
    const response = await fetch(
      `http://127.0.0.1:${port}/loadImage?path=${encodeURIComponent(filePath)}`
    )
    const blob = await response.blob()
    const b64 = (await blobToBase64(blob)) as string
    setAvatarPreview(
      `data:image/png;base64,${b64.substring(b64.indexOf(",") + 1)}`
    )

    // Upload immediately
    const uuid = globalStore?.currentlySelectedAccountUuid?.data
    if (uuid) {
      setAvatarLoading(true)
      try {
        await uploadAvatarMutation.mutateAsync({ uuid, iconPath: filePath })
        toast.success(t("accounts:_trn_avatar_upload_success"))
      } catch (err) {
        console.error("Avatar upload failed:", err)
        toast.error(t("accounts:_trn_avatar_upload_failed"))
      } finally {
        setAvatarLoading(false)
      }
    }
  }

  const handleAvatarDelete = async () => {
    const uuid = globalStore?.currentlySelectedAccountUuid?.data
    if (!uuid) return

    setAvatarLoading(true)
    try {
      await deleteAvatarMutation.mutateAsync(uuid)
      setAvatarPreview(null)
      toast.success(t("accounts:_trn_avatar_delete_success"))
    } catch (err) {
      console.error("Avatar deletion failed:", err)
      toast.error(t("accounts:_trn_avatar_delete_failed"))
    } finally {
      setAvatarLoading(false)
    }
  }

  const deleteAccountContent = () => {
    if (validGDLUser()?.deletionTimeout) {
      return (
        <Trans
          key="accounts:_trn_cannot_request_deletion_for_time"
          options={{
            time: convertSecondsToHumanTime(validGDLUser()?.deletionTimeout!)
          }}
        />
      )
    } else {
      return undefined
    }
  }

  const verificationContent = () => {
    if (validGDLUser()?.verificationTimeout) {
      return (
        <Trans
          key="accounts:_trn_cannot_request_deletion_for_time"
          options={{
            time: convertSecondsToHumanTime(
              validGDLUser()?.verificationTimeout!
            )
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
      <RowsContainer>
        <Row forceContentBelow>
          <Title>
            <Trans key="accounts:_trn_gdl_account_title" />
          </Title>
          <div class="bg-darkSlate-700 mb-6 p-4">
            <Switch>
              <Match when={validGDLUser()}>
                <div class="flex flex-col gap-4">
                  <div class="flex items-center justify-between gap-2">
                    <div class="text-xl text-green-400">
                      <Trans key="accounts:_trn_gdl_account_synced" />
                    </div>

                    <Popover>
                      <PopoverTrigger>
                        <Button type="outline">
                          <div class="i-hugeicons:logout-01 block h-6 w-6" />
                          <Trans key="accounts:_trn_log_out_gdl_account" />
                        </Button>
                      </PopoverTrigger>
                      <PopoverContent>
                        <Button
                          type="secondary"
                          onClick={() => {
                            removeGDLAccountMutation.mutate(undefined)
                          }}
                        >
                          <div class="i-hugeicons:logout-01 block h-6 w-6" />
                          <Trans key="accounts:_trn_confirm" />
                        </Button>
                      </PopoverContent>
                    </Popover>
                  </div>
                  <Show
                    when={validGDLUser() && !validGDLUser()?.isEmailVerified}
                  >
                    <div class="mb-4 flex items-center justify-between gap-8 rounded-md p-4 text-yellow-500 outline outline-yellow-500">
                      <div class="flex items-center gap-4">
                        <div class="i-hugeicons:alert-01 block h-6 w-6" />
                        <Trans key="accounts:_trn_gdl_account_not_verified" />
                      </div>
                      <Tooltip>
                        <TooltipTrigger>
                          <Button
                            disabled={!!validGDLUser()?.verificationTimeout}
                            onClick={async () => {
                              const uuid = globalStore.accounts.data?.find(
                                (account) =>
                                  account.uuid ===
                                  globalStore.settings.data?.gdlAccountId
                              )?.uuid

                              if (!uuid) {
                                throw new Error("No active gdl account")
                              }

                              const request =
                                await requestNewVerificationTokenMutation.mutateAsync(
                                  uuid
                                )

                              if (
                                request.status === "failed" &&
                                request.value
                              ) {
                                throw new Error(
                                  `Too many requests, retry in ${request.value}s`
                                )
                              }
                            }}
                          >
                            <div class="i-hugeicons:mail-send-01 text-lg" />
                            <Trans key="accounts:_trn_send_new_verification_email" />
                          </Button>
                        </TooltipTrigger>
                        <TooltipContent>{verificationContent()}</TooltipContent>
                      </Tooltip>
                    </div>
                  </Show>
                  <div class="grid grid-cols-2 gap-4">
                    <div class="flex items-center gap-4">
                      <ImagePicker
                        imageUrl={avatarPreview}
                        onSelect={handleAvatarSelect}
                        onDelete={handleAvatarDelete}
                        isLoading={avatarLoading}
                        deletable={!!validGDLUser()?.hasCustomAvatar}
                        confirmDelete={true}
                        sizeClass="h-12 w-12"
                        class="rounded-md"
                        dialogTitle={t("accounts:_trn_select_avatar_image")}
                      />
                      <GDLAccountRowItem
                        title={t("accounts:_trn_nickname")}
                        value={validGDLUser()?.nickname}
                        onEdit={() => {
                          modalsContext?.openModal({
                            name: "changeGDLAccountNickname"
                          })
                        }}
                        extraAction={
                          <Show
                            when={
                              nicknameHistoryQuery.data &&
                              nicknameHistoryQuery.data.length > 0
                            }
                          >
                            <div onClick={(e) => e.stopPropagation()}>
                              <Popover>
                                <PopoverTrigger>
                                  <div class="text-md text-lightSlate-700 hover:text-lightSlate-50 underline transition-all duration-100 ease-spring">
                                    <Trans key="accounts:_trn_nickname_history" />
                                  </div>
                                </PopoverTrigger>
                                <PopoverContent class="max-h-60 w-64 overflow-y-auto">
                                  <div class="flex flex-col gap-2">
                                    <div class="text-lightSlate-50 font-medium">
                                      <Trans key="accounts:_trn_nickname_history" />
                                    </div>
                                    <For each={nicknameHistoryQuery.data}>
                                      {(entry) => (
                                        <div class="text-lightSlate-300 flex justify-between text-sm">
                                          <span>{entry.nickname}</span>
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
                                          globalStore
                                            ?.currentlySelectedAccountUuid?.data
                                        if (!uuid) return

                                        try {
                                          await clearNicknameHistoryMutation.mutateAsync(
                                            uuid
                                          )
                                          queryClient.invalidateQueries({
                                            queryKey: [
                                              "account.getNicknameHistory"
                                            ]
                                          })
                                          toast.success(
                                            t(
                                              "accounts:_trn_nickname_history_cleared"
                                            )
                                          )
                                        } catch (err) {
                                          console.error(err)
                                          toast.error(
                                            t(
                                              "accounts:_trn_nickname_history_clear_failed"
                                            )
                                          )
                                        }
                                      }}
                                    >
                                      <Trans key="accounts:_trn_clear_nickname_history" />
                                    </Button>
                                  </div>
                                </PopoverContent>
                              </Popover>
                            </div>
                          </Show>
                        }
                      />
                    </div>
                    <GDLAccountRowItem
                      title={t("accounts:_trn_friend_code")}
                      value={validGDLUser()?.friendCode}
                    />
                    <GDLAccountRowItem
                      title={t("accounts:_trn_microsoft_username")}
                      value={
                        globalStore.accounts.data?.find(
                          (account) =>
                            account.uuid ===
                            globalStore.settings.data?.gdlAccountId
                        )?.username
                      }
                    />
                    <GDLAccountRowItem
                      title={t("accounts:_trn_microsoft_oid")}
                      value={validGDLUser()?.microsoftOid}
                    />
                    <GDLAccountRowItem
                      title={t("accounts:_trn_recovery_email")}
                      value={validGDLUser()?.email}
                      onEdit={() => {
                        modalsContext?.openModal({
                          name: "changeGDLAccountRecoveryEmail"
                        })
                      }}
                    />
                    <GDLAccountRowItem
                      title={t("accounts:_trn_microsoft_email")}
                      value={validGDLUser()?.microsoftEmail}
                    />
                  </div>
                </div>

                <div class="my-10 flex items-center gap-2 text-xl text-red-500">
                  <div class="i-hugeicons:alert-01 h-4 w-4" />
                  <Trans key="accounts:_trn_danger_zone" />
                </div>
                <div class="text-lightSlate-700 flex items-center justify-between gap-12">
                  <div>
                    <Trans key="accounts:_trn_request_account_deletion_description" />
                  </div>
                  <Tooltip>
                    <TooltipTrigger>
                      <Button
                        variant="red"
                        size="large"
                        disabled={!!validGDLUser()?.deletionTimeout}
                        onClick={() => {
                          modalsContext?.openModal({
                            name: "confirmGDLAccountDeletion"
                          })
                        }}
                      >
                        <div class="i-hugeicons:delete-02 block h-6 w-6" />
                        <Trans key="accounts:_trn_request_account_deletion" />
                      </Button>
                    </TooltipTrigger>
                    <TooltipContent>{deleteAccountContent()}</TooltipContent>
                  </Tooltip>
                </div>
              </Match>
              <Match when={!validGDLUser() && !invalidGDLUser()}>
                <div class="flex items-center justify-between gap-2">
                  <div class="text-xl text-red-400">
                    <Trans key="accounts:_trn_gdl_account_not_synced" />
                  </div>

                  <Button
                    type="outline"
                    onClick={async () => {
                      await removeGDLAccountMutation.mutateAsync(undefined)
                      gdNavigator.navigate(
                        "/?addGdlAccount=true&returnTo=/settings/accounts"
                      )
                    }}
                  >
                    <div class="i-hugeicons:link-01 text-lg" />
                    <Trans key="accounts:_trn_link_gdl_account" />
                  </Button>
                </div>
              </Match>
              <Match when={invalidGDLUser()}>
                <div class="flex items-center justify-between gap-2">
                  <div class="text-xl text-yellow-400">
                    <Trans key="accounts:_trn_gdl_account_error" />
                  </div>

                  <Button
                    type="outline"
                    onClick={() => {
                      removeGDLAccountMutation.mutate(undefined)
                    }}
                  >
                    <div class="i-hugeicons:logout-01 block h-6 w-6" />
                    <Trans key="accounts:_trn_log_out_gdl_account" />
                  </Button>
                </div>
              </Match>
            </Switch>
          </div>
        </Row>
      </RowsContainer>
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
