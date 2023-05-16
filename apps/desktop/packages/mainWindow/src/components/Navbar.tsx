import { Link, useLocation, useMatch, useRouteData } from "@solidjs/router";
import { For, Show, createEffect, createSignal } from "solid-js";
import GDLauncherWideLogo from "/assets/images/gdlauncher_wide_logo_blue.svg";
import { NAVBAR_ROUTES } from "@/constants";
import { Tab, TabList, Tabs, Spacing } from "@gd/ui";
import getRouteIndex from "@/route/getRouteIndex";
import { useGDNavigate } from "@/managers/NavigationManager";
import fetchData from "@/pages/app.data";
import { AccountsDropdown } from "./AccountsDropdown";
import { AccountType, Procedures } from "@gd/core_module/bindings";
import { createStore } from "solid-js/store";
import { port, rspc } from "@/utils/rspcClient";
import { useModal } from "@/managers/ModalsManager";

type EnrollStatusResult = Extract<
  Procedures["queries"],
  { key: "account.getAccountStatus" }
>["result"];

interface AccountsStatus {
  label: {
    name: string;
    icon: string | undefined;
    uuid: string;
    type: AccountType;
    status: EnrollStatusResult | undefined;
  };
  key: string;
}

const AppNavbar = () => {
  const location = useLocation();
  const navigate = useGDNavigate();
  const modalsContext = useModal();
  const [accounts, setAccounts] = createStore<AccountsStatus[]>([]);
  const [activeInstances, setActiveInstances] = createSignal(0);

  const isLogin = useMatch(() => "/");
  const isSettings = useMatch(() => "/settings");
  const isSettingsNested = useMatch(() => "/settings/*");

  const instances = rspc.createQuery(() => ["instance.getInstancesUngrouped"]);

  const selectedIndex = () =>
    !!isSettings() || !!isSettingsNested()
      ? 4
      : getRouteIndex(NAVBAR_ROUTES, location.pathname);

  const routeData = useRouteData<typeof fetchData>();

  createEffect(() => {
    const mappedAccounts = routeData.accounts.data?.map((account) => {
      const accountStatusQuery = rspc.createQuery(() => [
        "account.getAccountStatus",
        account.uuid,
      ]);

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

  // createSignal(() => {
  //   instances.data?.forEach((instance) => {});
  // });

  return (
    <Show when={!isLogin()}>
      <nav class="flex items-center text-white px-5 bg-dark-slate-800 h-15">
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
                            <li class="no-underline">{route.label}</li>
                          </Tab>
                        </div>
                      );
                    }}
                  </For>
                </div>
                <Spacing class="w-full" />
                <div class="flex gap-6 items-center">
                  <Tab ignored>
                    <div class="relative">
                      <div class="absolute w-4 h-4 -top-1 -right-1 rounded-full bg-red-500 z-30 text-white flex justify-center items-center text-xs">
                        2
                      </div>
                      <div
                        class="cursor-pointer text-2xl text-dark-slate-50 i-ri:terminal-box-fill z-20"
                        onClick={() =>
                          modalsContext?.openModal({ name: "logViewer" })
                        }
                      />
                    </div>
                  </Tab>
                  <Link href="/settings" class="no-underline">
                    <Tab>
                      <div
                        class="text-dark-slate-50 text-2xl cursor-pointer i-ri:settings-3-fill"
                        classList={{
                          "bg-primary-500":
                            !!isSettings() || !!isSettingsNested(),
                        }}
                      />
                    </Tab>
                  </Link>
                  <div
                    class="text-dark-slate-50 text-2xl cursor-pointer i-ri:notification-2-fill"
                    onClick={() =>
                      modalsContext?.openModal({ name: "notification" })
                    }
                  />
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
