import { useLocation, useMatch, useRouteData } from "@solidjs/router";
import { For, Show, createEffect } from "solid-js";
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
import updateAvailable from "@/utils/updater";
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
      ? 5
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
          status: accountStatusQuery.data
        },
        key: account?.uuid
      };
    });

    if (mappedAccounts) {
      setAccounts(mappedAccounts);
    }
  });

  return (
    <Show when={!isLogin()}>
      <nav class="flex items-center bg-darkSlate-800 text-white px-5 h-20">
        <div class="flex items-center" style={{ width: "19rem" }}>
          <img
            src={GDLauncherWideLogo}
            class="h-9"
            onClick={() => navigate("/library")}
          />
        </div>
        <div class="flex text-white w-full items-center list-none gap-6 h-full">
          <Tabs index={selectedIndex()}>
            <TabList aligment="between">
              <div class="flex gap-6 h-full">
                <For each={NAVBAR_ROUTES}>
                  {(route) => {
                    return (
                      <Tab
                        onClick={() =>
                          navigate(route.path, {
                            getLastInstance: true
                          })
                        }
                      >
                        <div class="flex items-center gap-2">
                          <Show when={route.icon}>
                            <i class={"w-5 h-5 " + route.icon} />
                          </Show>
                          <div class="no-underline">{route.label}</div>
                        </div>
                      </Tab>
                    );
                  }}
                </For>
              </div>
              <Spacing class="hidden w-full lg:block" />
              <Tab ignored noPadding>
                <Button
                  class="w-max"
                  size="small"
                  type="primary"
                  onClick={() => {
                    modalsContext?.openModal({
                      name: "instanceCreation"
                    });
                  }}
                >
                  <i class="i-ri:add-fill" />
                  <Trans key="sidebar.add_instance" />
                </Button>
              </Tab>

              <div class="flex gap-6 items-center">
                <div
                  onClick={() => {
                    if (!(!!isSettings() || !!isSettingsNested()))
                      navigate("/settings", {
                        getLastInstance: true
                      });
                  }}
                >
                  <Tab>
                    <div
                      class="text-2xl i-ri:settings-3-fill"
                      classList={{
                        "text-white": !!isSettings() || !!isSettingsNested()
                      }}
                    />
                  </Tab>
                </div>
                <Show when={updateAvailable()}>
                  <Tab ignored>
                    <div
                      class="text-2xl text-green-600 i-ri:download-2-fill"
                      onClick={() =>
                        modalsContext?.openModal({ name: "appUpdate" })
                      }
                    />
                  </Tab>
                </Show>
              </div>
            </TabList>
          </Tabs>
        </div>
        <div class="flex justify-end lg:min-w-52 lg:ml-4 ml-2">
          <Show when={routeData?.accounts.data}>
            <AccountsDropdown
              accounts={accounts}
              value={routeData.activeUuid.data}
            />
          </Show>
        </div>
      </nav>
    </Show>
  );
};

export default AppNavbar;
