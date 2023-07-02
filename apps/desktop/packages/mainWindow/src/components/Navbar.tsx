import { useLocation, useMatch, useRouteData } from "@solidjs/router";
import { For, Show, createEffect, onMount } from "solid-js";
import GDLauncherWideLogo from "/assets/images/gdlauncher_wide_logo_blue.svg";
import { NAVBAR_ROUTES } from "@/constants";
import { Tab, TabList, Tabs, Spacing, Button } from "@gd/ui";
import getRouteIndex from "@/route/getRouteIndex";
import { useGDNavigate } from "@/managers/NavigationManager";
import fetchData from "@/pages/app.data";
import { AccountsDropdown } from "./AccountsDropdown";
import { AccountStatus, AccountType } from "@gd/core_module/bindings";
import { createStore } from "solid-js/store";
import { port } from "@/utils/rspcClient";
import { checkForUpdates } from "@/utils/updaterhelper";
import { Trans } from "@gd/i18n";
import { useModal } from "@/managers/ModalsManager";

interface AccountsStatus {
  label: {
    name: string;
    icon: string | undefined;
    uuid: string;
    type: AccountType;
    status: AccountStatus | undefined;
  };
  key: string;
}

const AppNavbar = () => {
  const location = useLocation();
  const navigate = useGDNavigate();
  const [accounts, setAccounts] = createStore<AccountsStatus[]>([]);
  const modalsContext = useModal();

  const isLogin = useMatch(() => "/");
  const isSettings = useMatch(() => "/settings");
  const isSettingsNested = useMatch(() => "/settings/*");

  const selectedIndex = () =>
    !!isSettings() || !!isSettingsNested()
      ? 4
      : getRouteIndex(NAVBAR_ROUTES, location.pathname);

  const routeData = useRouteData<typeof fetchData>();

  createEffect(() => {
    const mappedAccounts = routeData.accounts.data?.map((account) => {
      const accountStatusQuery = {} as any;

      return {
        label: {
          name: account?.username,
          icon: `http://localhost:${port}/account/headImage?uuid=${account.uuid}`,
          uuid: account.uuid,
          type: account.type,
          status: accountStatusQuery.data,
        },
        key: account?.uuid,
      };
    });

    if (mappedAccounts) {
      setAccounts(mappedAccounts);
    }
  });

  onMount(() => {
    checkForUpdates();
  });

  return (
    <Show when={!isLogin()}>
      <nav class="flex items-center text-white bg-darkSlate-800 px-5 h-15">
        <div class="flex w-full">
          <div class="flex items-center w-36">
            <img
              src={GDLauncherWideLogo}
              class="cursor-pointer h-9"
              onClick={() => navigate("/library")}
            />
          </div>
          <ul class="flex text-white w-full m-0 list-none items-between gap-6 pl-10">
            <Tabs index={selectedIndex()}>
              <TabList aligment="between">
                <div class="flex gap-6">
                  <For each={NAVBAR_ROUTES}>
                    {(route) => {
                      const isMatch = useMatch(() => route.path);

                      return (
                        <div
                          onClick={() =>
                            navigate(route.path, {
                              getLastInstance: true,
                            })
                          }
                          class="no-underline"
                          classList={{
                            "text-white": !!isMatch(),
                            "text-slate-400": !isMatch(),
                          }}
                        >
                          <Tab>
                            <div class="flex items-center gap-2">
                              <i class={"w-5 h-5 " + route.icon} />
                              <li class="no-underline">{route.label}</li>
                            </div>
                          </Tab>
                        </div>
                      );
                    }}
                  </For>
                </div>
                <Spacing class="w-full" />
                <Tab ignored noPadding>
                  <Button
                    size="small"
                    class="w-max"
                    type="primary"
                    onClick={() => {
                      modalsContext?.openModal({
                        name: "instanceCreation",
                      });
                    }}
                  >
                    <Trans
                      key="sidebar.add_instance"
                      options={{
                        defaultValue: "Add Instance",
                      }}
                    />
                  </Button>
                </Tab>
                <div class="flex gap-6 items-center">
                  <div
                    onClick={() =>
                      navigate("/settings", {
                        getLastInstance: true,
                      })
                    }
                  >
                    <Tab>
                      <div
                        class="text-darkSlate-50 text-2xl cursor-pointer i-ri:settings-3-fill"
                        classList={{
                          "bg-primary-500":
                            !!isSettings() || !!isSettingsNested(),
                        }}
                      />
                    </Tab>
                  </div>
                  {/* <div
                    class="text-2xl cursor-pointer text-dark-slate-50 i-ri:notification-2-fill"
                    onClick={() =>
                      modalsContext?.openModal({ name: "notification" })
                    }
                  /> */}
                </div>
              </TabList>
            </Tabs>
          </ul>
          <div class="flex justify-end ml-4 min-w-52">
            <Show when={routeData?.accounts.data}>
              <AccountsDropdown
                accounts={accounts}
                value={routeData.activeUuid.data}
              />
            </Show>
          </div>
        </div>
      </nav>
    </Show>
  );
};

export default AppNavbar;
