import {
  flexRender,
  getCoreRowModel,
  ColumnDef,
  createSolidTable
} from "@tanstack/solid-table";
import { Trans, useTransContext } from "@gd/i18n";
import { Button, createNotification, Tooltip } from "@gd/ui";
import { port, rspc } from "@/utils/rspcClient";
import PageTitle from "./components/PageTitle";
import Row from "./components/Row";
import Title from "./components/Title";
import RowsContainer from "./components/RowsContainer";
import { useGlobalStore } from "@/components/GlobalStoreContext";
import { For, JSX, Match, Show, Switch } from "solid-js";
import { useGDNavigate } from "@/managers/NavigationManager";
import { convertSecondsToHumanTime } from "@/utils/helpers";
import { useModal } from "@/managers/ModalsManager";
import { AccountEntry } from "@gd/core_module/bindings";

const GDLAccountRow = (props: { children: JSX.Element }) => {
  return (
    <div class="flex items-center justify-between gap-4">{props.children}</div>
  );
};

const GDLAccountRowItem = (props: {
  title?: string;
  value?: string | null | undefined;
  children?: JSX.Element;
  onEdit?: () => void;
}) => {
  return (
    <div class="flex justify-between items-center">
      <div class="flex flex-col gap-2 justify-center">
        <Show when={props.title}>
          <div class="text-md font-bold text-lightSlate-600 uppercase">
            {props.title}
          </div>
        </Show>
        <Show when={props.value}>
          <div class="text-lightSlate-50 text-ellipsis overflow-hidden whitespace-nowrap">
            {props.value}
          </div>
        </Show>
        {props.children}
      </div>
      <Show when={props.onEdit}>
        <div class="text-md underline">EDIT</div>
      </Show>
    </div>
  );
};

const defaultColumns: ColumnDef<AccountEntry>[] = [
  {
    accessorFn: (row) => row.username,
    id: "username",
    cell: (info) => (
      <div class="flex gap-4 items-center">
        <img
          src={`http://127.0.0.1:${port}/account/headImage?uuid=${info.row.original.uuid}`}
          class="w-8 h-8 rounded-md"
        />
        {info.row.original.username}
      </div>
    ),
    header: () => <span>Username</span>
  },
  {
    accessorFn: (row) => row.type.type,
    id: "type",
    cell: (info) => info.getValue(),
    header: () => <span>Type</span>
  },
  {
    accessorFn: (row) => row.uuid,
    id: "uuid",
    cell: (info) => info.getValue(),
    header: () => <span>UUID</span>
  },
  {
    accessorFn: (row) => <></>,
    id: "actions",
    cell: (info) => info.getValue(),
    header: () => <span>Actions</span>
  }
];

const Accounts = () => {
  const globalStore = useGlobalStore();
  const [t] = useTransContext();

  const navigate = useGDNavigate();
  const modalsContext = useModal();
  const addNotification = createNotification();

  const removeGDLAccountMutation = rspc.createMutation(() => ({
    mutationKey: ["account.removeGdlAccount"]
  }));

  const requestNewVerificationTokenMutation = rspc.createMutation(() => ({
    mutationKey: ["account.requestNewVerificationToken"]
  }));

  const deleteAccountMutation = rspc.createMutation(() => ({
    mutationKey: ["account.deleteAccount"]
  }));

  const validGDLUser = () =>
    globalStore.gdlAccount.data?.status === "valid"
      ? globalStore.gdlAccount.data?.value
      : undefined;

  const deleteAccountContent = () => {
    if (validGDLUser()?.deletionTimeout) {
      return (
        <Trans
          key="settings:cannot_request_deletion_for_time"
          options={{
            time: convertSecondsToHumanTime(validGDLUser()?.deletionTimeout!)
          }}
        />
      );
    } else {
      return undefined;
    }
  };

  const verificationContent = () => {
    if (validGDLUser()?.verificationTimeout) {
      return (
        <Trans
          key="settings:cannot_request_deletion_for_time"
          options={{
            time: convertSecondsToHumanTime(
              validGDLUser()?.verificationTimeout!
            )
          }}
        />
      );
    } else {
      return undefined;
    }
  };

  const accountsTable = createSolidTable({
    get data() {
      return globalStore.accounts.data || [];
    },
    columns: defaultColumns,
    getCoreRowModel: getCoreRowModel()
  });

  return (
    <>
      <PageTitle>
        <Trans key="settings:accounts" />
      </PageTitle>
      <RowsContainer>
        <Row forceContentBelow>
          <Title>
            <Trans key="settings:gdl_account_title" />
          </Title>
          <div class="bg-darkSlate-700 p-4 mb-6">
            <Switch>
              <Match when={validGDLUser()}>
                <div class="flex flex-col gap-4">
                  <div class="flex gap-2 items-center justify-between">
                    <div class="text-green-400 text-xl">
                      <Trans key="settings:gdl_account_synced" />
                    </div>

                    <Button
                      type="outline"
                      onClick={() => {
                        removeGDLAccountMutation.mutate(undefined);
                      }}
                    >
                      <Trans key="settings:log_out_gdl_account" />
                    </Button>
                  </div>
                  <Show when={!validGDLUser()?.isEmailVerified}>
                    <div class="flex items-center justify-between outline outline-yellow-500 text-yellow-500 p-4 rounded-md mb-4">
                      <div class="flex items-center gap-4">
                        <i class="block w-6 h-6 i-ri:alert-fill" />
                        <Trans key="settings:gdl_account_not_verified" />
                      </div>
                      <Tooltip content={verificationContent()}>
                        <Button
                          disabled={!!validGDLUser()?.verificationTimeout}
                          onClick={async () => {
                            const uuid = globalStore.accounts.data?.find(
                              (account) =>
                                account.uuid ===
                                globalStore.settings.data?.gdlAccountId
                            )?.uuid;

                            if (!uuid) {
                              throw new Error("No active gdl account");
                            }

                            const request =
                              await requestNewVerificationTokenMutation.mutateAsync(
                                uuid
                              );

                            if (request.status === "failed" && request.value) {
                              throw new Error(
                                `Too many requests, retry in ${request.value}s`
                              );
                            }
                          }}
                        >
                          <Trans key="settings:send_new_verification_email" />
                        </Button>
                      </Tooltip>
                    </div>
                  </Show>
                  <div class="grid grid-cols-2 gap-4">
                    <GDLAccountRowItem
                      title={t("settings:minecraft_uuid")}
                      value={
                        globalStore.accounts.data?.find(
                          (account) =>
                            account.uuid ===
                            globalStore.settings.data?.gdlAccountId
                        )?.uuid
                      }
                    />
                    <GDLAccountRowItem
                      title={t("settings:microsoft_username")}
                      value={
                        globalStore.accounts.data?.find(
                          (account) =>
                            account.uuid ===
                            globalStore.settings.data?.gdlAccountId
                        )?.username
                      }
                    />
                    <GDLAccountRowItem
                      title={t("settings:recovery_email")}
                      value={validGDLUser()?.email}
                      onEdit={() => {}}
                    />
                    <GDLAccountRowItem
                      title={t("settings:microsoft_email")}
                      value={validGDLUser()?.microsoftEmail}
                    />
                  </div>
                </div>

                <div class="my-10 text-red-500 text-xl">
                  <Trans key="settings:danger_zone" />
                </div>
                <div class="flex items-center justify-between gap-12 text-lightSlate-800">
                  <div>
                    <Trans key="settings:request_account_deletion_description" />
                  </div>
                  <Tooltip content={deleteAccountContent()}>
                    <Button
                      variant="red"
                      size="large"
                      disabled={!!validGDLUser()?.deletionTimeout}
                      onClick={() => {
                        modalsContext?.openModal({
                          name: "confirmGDLAccountDeletion"
                        });
                      }}
                    >
                      <Trans key="settings:request_account_deletion" />
                    </Button>
                  </Tooltip>
                </div>
              </Match>
              <Match when={!validGDLUser()}>
                <div class="flex gap-2 items-center justify-between">
                  <div class="text-red-400 text-xl">
                    <Trans key="settings:gdl_account_not_synced" />
                  </div>

                  <Button
                    type="outline"
                    onClick={async () => {
                      await removeGDLAccountMutation.mutateAsync(undefined);
                      navigate("/");
                    }}
                  >
                    <Trans key="settings:add_gdl_account" />
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
              <Trans key="settings:minecraft_accounts" />
              <Button
                type="secondary"
                size="small"
                onClick={() => {
                  navigate("/?addMicrosoftAccount=true");
                }}
              >
                <div class="i-ri:add-line" />
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
                        class={`font-bold text-lightSlate-900 border-0 border-darkSlate-500 border-solid ${i() !== 0 ? "border-l-1" : ""}`}
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
                <tr class="hover:bg-darkSlate-700 transition-colors duration-100 ease-in-out group/external">
                  <For each={row.getVisibleCells()}>
                    {(cell, i) => (
                      <td
                        class={`group/internal text-lightSlate-300 hover:text-white relative p-4 border-0 border-transparent group-hover/external:border-darkSlate-500 border-solid ${i() !== 0 ? "border-l-1" : ""}`}
                        onClick={() => {
                          if (cell.column.columnDef.id !== "actions") {
                            navigator.clipboard.writeText(
                              cell.getValue() as string
                            );

                            addNotification({
                              name: "Copied to clipboard",
                              type: "success"
                            });
                          }
                        }}
                      >
                        <Show when={cell.column.columnDef.id === "actions"}>
                          <div class="flex gap-4 items-center">
                            <div
                              class="i-ri:delete-bin-2-fill"
                              onClick={async () => {
                                const accountsLength =
                                  globalStore.accounts.data?.length;
                                await deleteAccountMutation.mutateAsync(
                                  (row.original as AccountEntry).uuid
                                );

                                if (accountsLength === 1) {
                                  navigate("/");
                                }
                              }}
                            />
                          </div>
                        </Show>
                        <Show when={cell.column.columnDef.id !== "actions"}>
                          <div class="absolute right-4 top-1/2 -translate-y-1/2 opacity-0 group-hover/internal:opacity-100 duration-100 ease-in-out">
                            <div class="i-ri:clipboard-fill text-white" />
                          </div>
                        </Show>
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
  );
};

export default Accounts;
