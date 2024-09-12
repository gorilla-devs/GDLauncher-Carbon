import { useGDNavigate } from "@/managers/NavigationManager";

import { port, rspc } from "@/utils/rspcClient";
import { AccountStatus, AccountType } from "@gd/core_module/bindings";
import { Trans } from "@gd/i18n";
import { Button, Popover } from "@gd/ui";
import { For, Switch, Match } from "solid-js";
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

  let gdlAccountRef: HTMLDivElement;
  let mcAccountsRef: HTMLDivElement;

  return (
    <Popover
      placement="bottom"
      color="bg-transparent"
      noTip
      noShadow
      trigger="click"
      onOpen={() => {
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
          );
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
              delay: 100
            }
          );
        }
      }}
      content={(close) => (
        <div class="flex flex-col gap-2">
          <div
            class="bg-darkSlate-700 w-70 h-auto p-2 rounded-lg mr-2 opacity-0 shadow-lg shadow-darkSlate-900 transition-opacity"
            ref={gdlAccountRef}
          >
            <div class="p-4 text-xl flex items-center gap-4">
              <img src="/assets/images/gdlauncher_logo.svg" class="w-6 h-6" />
              <div>
                <Trans key="GDLauncher_account" />
              </div>
            </div>
            <hr class="w-full border-darkSlate-50 opacity-20" />
            <Switch
              fallback={
                <div class="flex items-center gap-4 p-4 rounded-lg">
                  <div class="w-6 h-6 rounded-md bg-darkSlate-600" />
                  <div>
                    <Trans key="No_account_synced" />
                  </div>
                </div>
              }
            >
              <Match when={validGDLUser()}>
                <div class="flex items-center gap-4 p-4 rounded-lg">
                  <img
                    src={`http://127.0.0.1:${port}/account/headImage?uuid=${
                      globalStore.accounts.data?.find(
                        (account) =>
                          account.uuid ===
                          globalStore.settings.data?.gdlAccountId
                      )?.uuid
                    }`}
                    class="w-6 h-6 rounded-md"
                  />
                  <div class="truncate max-w-30">
                    {
                      globalStore.accounts.data?.find(
                        (account) =>
                          account.uuid ===
                          globalStore.settings.data?.gdlAccountId
                      )?.username
                    }
                  </div>
                </div>
              </Match>
            </Switch>
          </div>
          <div
            class="bg-darkSlate-700 w-70 h-auto p-2 rounded-lg mr-2 opacity-0 shadow-lg shadow-darkSlate-900 transition-opacity"
            ref={mcAccountsRef}
          >
            <div class="p-4 text-xl flex items-center gap-4">
              <img
                src="/assets/images/default-instance-img.png"
                class="w-6 h-6"
              />
              <div>
                <Trans key="Minecraft_accounts" />
              </div>
            </div>
            <hr class="w-full border-darkSlate-50 opacity-20" />
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
              fullWidth
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
          <div class="w-4 h-4 i-ri:arrow-down-s-line" />
        </div>
      </div>
    </Popover>
  );
};
