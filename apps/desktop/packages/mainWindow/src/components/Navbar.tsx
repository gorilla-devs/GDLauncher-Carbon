import { Link, useLocation, useMatch, useRouteData } from "@solidjs/router";
import { For, Show, createEffect, onMount } from "solid-js";
import GDLauncherWideLogo from "/assets/images/gdlauncher_wide_logo_blue.svg";
import ProfileImg from "/assets/images/profile-img.png";
// import ProfileImg2 from "/assets/images/profile-img2.png";
import { NAVBAR_ROUTES } from "@/constants";
import { Tab, TabList, Tabs, Spacing, Dropdown } from "@gd/ui";
import getRouteIndex from "@/route/getRouteIndex";
import { useGDNavigate } from "@/managers/NavigationManager";
import fetchData from "@/pages/app.data";
import { rspc } from "@/utils/rspcClient";
import { createStore } from "solid-js/store";
import { AccountsDropdown } from "./AccountsDropdown";
import { AccountType } from "@gd/core_module/bindings";

export interface AccountsStatus {
  [details: string]: {
    username: string;
    uuid: string;
    type_: AccountType;
    status: "Ok" | "Expired" | "Refreshing" | null | undefined;
  };
}

const AppNavbar = () => {
  const [accountsStatus, setAccountsStatus] = createStore<AccountsStatus>({});
  const location = useLocation();
  const navigate = useGDNavigate();

  const isLogin = useMatch(() => "/");
  const isSettings = useMatch(() => "/settings");
  const isSettingsNested = useMatch(() => "/settings/*");

  const selectedIndex = () =>
    !!isSettings() || !!isSettingsNested()
      ? 4
      : getRouteIndex(NAVBAR_ROUTES, location.pathname);

  const accounts = useRouteData<typeof fetchData>();

  createEffect(() => {
    if (accounts.data) {
      console.log("accounts.data", accounts);
      for (let index = 0; index < accounts.data.length; index++) {
        const element = accounts.data[index];

        let accountStatus = rspc.createQuery(() => [
          "account.getAccountStatus",
          element.uuid,
        ]);

        setAccountsStatus((prev) => ({
          ...prev,
          [element.uuid]: {
            ...element,
            status: accountStatus.data,
          },
        }));
      }
    }
  });
  createEffect(() => {
    console.log("TEST", accountsStatus);
  });

  return (
    <Show when={!isLogin()}>
      <nav class="bg-shade-8 flex items-center text-white px-5 h-15">
        <div class="flex w-full items-center">
          <div class="flex items-center w-36">
            <img
              src={GDLauncherWideLogo}
              class="cursor-pointer h-9"
              onClick={() => navigate("/library")}
            />
          </div>
          <ul class="flex text-white w-full m-0 items-between gap-6 list-none pl-10">
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
                    <div class="cursor-pointer text-shade-0 text-2xl i-ri:terminal-box-fill" />
                  </Tab>
                  <Link href="/settings" class="no-underline">
                    <Tab>
                      <div
                        class="text-shade-0 text-2xl cursor-pointer i-ri:settings-3-fill"
                        classList={{
                          "bg-primary": !!isSettings() || !!isSettingsNested(),
                        }}
                      />
                    </Tab>
                  </Link>
                  <div class="text-shade-0 text-2xl cursor-pointer i-ri:notification-2-fill" />
                </div>
              </TabList>
            </Tabs>
          </ul>
          <div class="ml-4">
            <Show when={accounts.data && accounts?.data?.length > 0}>
              <AccountsDropdown
                options={(accounts.data || []).map((account) => ({
                  label: {
                    name: account?.username,
                    icon: ProfileImg,
                    uuid: account.uuid,
                    status: accountsStatus[account.uuid]?.status,
                    type: account.type_,
                  },
                  key: account?.uuid,
                }))}
                value="ladvace"
              />
            </Show>
          </div>
        </div>
      </nav>
    </Show>
  );
};

export default AppNavbar;
