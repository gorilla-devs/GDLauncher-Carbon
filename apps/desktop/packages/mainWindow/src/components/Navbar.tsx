import { useLocation, useMatch, useRouteData } from "@solidjs/router";
import { For, Match, Show, Switch, createEffect } from "solid-js";
import GDLauncherWideLogo from "/assets/images/gdlauncher_wide_logo_blue.svg";
import { NAVBAR_ROUTES } from "@/constants";
import { Tab, TabList, Tabs, Spacing, Button, Tooltip } from "@gd/ui";
import getRouteIndex from "@/route/getRouteIndex";
import { useGDNavigate } from "@/managers/NavigationManager";
import fetchData from "@/pages/app.data";
import { AccountsDropdown } from "./AccountsDropdown";
import { AccountStatus, AccountType } from "@gd/core_module/bindings";
import { createStore } from "solid-js/store";
import { port, rspc } from "@/utils/rspcClient";
import updateAvailable, {
  updateDownloaded,
  updateProgress
} from "@/utils/updater";
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

  const blockingMutation = rspc.createMutation(() => ({
    mutationKey: "longRunning"
  }));

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
          icon: `http://127.0.0.1:${port}/account/headImage?uuid=${account.uuid}`,
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
      <nav
        class="flex items-center bg-darkSlate-800 text-white px-5"
        style={{
          height: "60px"
        }}
      >
        <div class="flex items-center" style={{ width: "19rem" }}>
          <img
            src={GDLauncherWideLogo}
            class="h-9"
            onClick={() => navigate("/library")}
          />
        </div>
        <div class="flex text-white w-full items-center h-full list-none gap-6">
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
                  <i class="flex i-ri:add-fill" />
                  <Trans key="sidebar.add_instance" />
                </Button>
                <Button
                  class="w-max"
                  size="small"
                  type="primary"
                  onClick={async () => {
                    console.log("BLOCKING MUTATION");
                    const x = await blockingMutation.mutateAsync(undefined);
                    console.log("GOT IT MUTATION", x);
                  }}
                >
                  <i class="flex i-ri:add-fill" />
                  BLOCKING MUTATION
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
                <Show
                  when={
                    updateAvailable() ||
                    updateDownloaded() ||
                    updateProgress() > 0
                  }
                >
                  <Tab ignored>
                    <Tooltip
                      placement="bottom"
                      content={
                        <Switch>
                          <Match when={updateDownloaded()}>
                            <Trans key="app_update.apply_and_restart" />
                          </Match>
                          <Match when={updateProgress() > 0}>
                            <Trans
                              key="app_update.downloading"
                              options={{
                                progress: updateProgress()
                              }}
                            />
                          </Match>
                          <Match when={updateAvailable()}>
                            <Trans key="app_update.new_update_available_text" />
                          </Match>
                        </Switch>
                      }
                    >
                      <div
                        class="text-2xl text-green-500 i-ri:download-2-fill"
                        classList={{
                          "hover:text-green-100": !updateDownloaded()
                        }}
                        onClick={() => {
                          if (updateDownloaded()) {
                            window.installUpdate();
                          } else {
                            modalsContext?.openModal({ name: "appUpdate" });
                          }
                        }}
                      />
                    </Tooltip>
                  </Tab>
                </Show>
              </div>
            </TabList>
          </Tabs>
        </div>
        <div class="flex ml-2 justify-end lg:min-w-52 lg:ml-4">
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
