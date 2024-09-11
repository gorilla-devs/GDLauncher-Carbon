import { useGDNavigate } from "@/managers/NavigationManager";
import fetchData from "@/pages/app.data";
import {
  msToMinutes,
  msToSeconds,
  parseTwoDigitNumber,
  strToMs
} from "@/utils/helpers";
import { handleStatus } from "@/utils/login";
import { port, queryClient, rspc } from "@/utils/rspcClient";
import {
  AccountEntry,
  AccountStatus,
  AccountType,
  DeviceCode
} from "@gd/core_module/bindings";
import { Trans } from "@gd/i18n";
import { Button, Popover, Spinner, createNotification } from "@gd/ui";
import { useRouteData } from "@solidjs/router";
import { createSignal, For, Show, Switch, Match, createEffect } from "solid-js";
import CopyIcon from "../CopyIcon";
import { useGlobalStore } from "../GlobalStoreContext";

export type Label = {
  name: string;
  icon: string | undefined;
  uuid: string;
  type: AccountType;
  status: AccountStatus | undefined;
};

export type Account = {
  label: Label;
  key: string;
};

export type OptionDropDown = {
  label: string;
  key: string;
};

export type Props = {
  accounts: Account[];
  value: string | null | undefined;
  disabled?: boolean;
  label?: string;
  id?: string;
};

const mapStatus = (status: AccountStatus | undefined) => {
  return (
    <Switch
      fallback={
        <div class="flex items-center gap-2">
          <div class="rounded-full w-3 h-3 text-yellow i-ri:alert-fill" />
          <p class="m-0 text-xs">
            <Trans
              key="account_invalid"
              options={{
                defaultValue: "invalid"
              }}
            />
          </p>
        </div>
      }
    >
      <Match when={status === "invalid"}>
        <div class="flex gap-2 items-center">
          <div class="w-3 h-3 rounded-full text-yellow i-ri:alert-fill" />
          <p class="m-0 text-xs">
            <Trans
              key="account_invalid"
              options={{
                defaultValue: "invalid"
              }}
            />
          </p>
        </div>
      </Match>
      <Match when={status === "ok"}>
        <div class="flex gap-2 items-center">
          <div class="w-3 h-3 rounded-full bg-green" />
          <p class="m-0 text-xs">
            <Trans
              key="account_online"
              options={{
                defaultValue: "online"
              }}
            />
          </p>
        </div>
      </Match>
      <Match when={status === "expired"}>
        <div class="flex gap-2 items-center">
          <div class="w-3 h-3 rounded-full bg-red" />
          <p class="m-0 text-xs">
            <Trans
              key="account_expired"
              options={{
                defaultValue: "Expired"
              }}
            />
          </p>
        </div>
      </Match>
      <Match when={status === "refreshing"}>
        <div class="flex flex gap-2 items-center">
          <div class="i-ri:refresh-line" />
          <Trans
            key="account_refreshing"
            options={{
              defaultValue: "Refresh"
            }}
          />
        </div>
      </Match>
    </Switch>
  );
};

const mapTypeToIcon = (type: string) => {
  return (
    <Switch>
      <Match when={type === "microsoft"}>
        <div class="i-ri:microsoft-fill" />
      </Match>
    </Switch>
  );
};

export const AccountsDropdown = (props: Props) => {
  const globalStore = useGlobalStore();
  const navigate = useGDNavigate();

  const setActiveAccountMutation = rspc.createMutation(() => ({
    mutationKey: ["account.setActiveUuid"]
  }));

  const validGDLUser = () =>
    globalStore.gdlAccount.data?.status === "valid"
      ? globalStore.gdlAccount.data?.value
      : undefined;

  return (
    <Popover
      placement="bottom"
      color="bg-transparent"
      noTip
      trigger="click"
      content={(close) => (
        <div class="flex flex-col gap-2">
          <div class="bg-darkSlate-700 w-50 h-auto p-2 rounded-lg mr-2">
            GDLauncher account
          </div>
          <div class="bg-darkSlate-700 w-50 h-auto p-2 rounded-lg mr-2">
            <For each={globalStore.accounts.data || []}>
              {(account) => (
                <div
                  class="flex items-center gap-4 p-4 hover:bg-darkSlate-600 rounded-lg"
                  classList={{
                    "bg-darkSlate-600":
                      account.uuid ===
                      globalStore.currentlySelectedAccountUuid.data
                  }}
                  onClick={() => {
                    setActiveAccountMutation.mutate(account.uuid);
                  }}
                >
                  <img
                    src={`http://127.0.0.1:${port}/account/headImage?uuid=${account.uuid}`}
                    class="w-6 h-6 rounded-md"
                  />
                  <div class="truncate max-w-30">{account.username}</div>
                </div>
              )}
            </For>

            <hr class="w-full border-darkSlate-50 opacity-20 my-4" />
            <Button
              type="outline"
              class="flex items-center justify-center gap-4 mb-2"
              onClick={() => {
                if (props.disabled) return;
                navigate("/settings/accounts");
                close();
              }}
            >
              <div
                class="text-2xl i-ri:settings-line pointer-events-auto"
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
      )}
    >
      <div class="bg-darkSlate-700 p-4 rounded-lg">
        <div class="flex gap-4 items-center">
          <img
            src={`http://127.0.0.1:${port}/account/headImage?uuid=${
              globalStore.accounts.data?.find(
                (account) =>
                  account.uuid === globalStore.currentlySelectedAccountUuid.data
              )?.uuid
            }`}
            class="w-6 h-6 rounded-md"
          />
          <div class="truncate max-w-30">
            {
              globalStore.accounts.data?.find(
                (account) =>
                  account.uuid === globalStore.currentlySelectedAccountUuid.data
              )?.username
            }
          </div>
        </div>
      </div>
    </Popover>
  );
};
