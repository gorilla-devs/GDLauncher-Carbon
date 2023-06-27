import { useGDNavigate } from "@/managers/NavigationManager";
import fetchData from "@/pages/app.data";
import {
  msToMinutes,
  msToSeconds,
  parseTwoDigitNumber,
  strToMs,
} from "@/utils/helpers";
import { handleStatus } from "@/utils/login";
import { queryClient, rspc } from "@/utils/rspcClient";
import {
  AccountEntry,
  AccountStatus,
  AccountType,
  DeviceCode,
} from "@gd/core_module/bindings";
import { Trans } from "@gd/i18n";
import { Spinner, createNotification } from "@gd/ui";
import { useRouteData } from "@solidjs/router";
import { createSignal, For, Show, Switch, Match, createEffect } from "solid-js";

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
                defaultValue: "invalid",
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
                defaultValue: "invalid",
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
                defaultValue: "online",
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
                defaultValue: "Expired",
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
              defaultValue: "Refresh",
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
      <Match when={type === "Microsoft"}>
        <div class="i-ri:microsoft-fill" />
      </Match>
    </Switch>
  );
};

export const AccountsDropdown = (props: Props) => {
  const activeAccount = () =>
    props.accounts.find((option) => option.key === props.value)?.label ||
    props.accounts[0]?.label;

  const [menuOpened, setMenuOpened] = createSignal(false);
  const [addAccountStarting, setAddAccountStarting] = createSignal(false);
  const [loginDeviceCode, setLoginDeviceCode] = createSignal<DeviceCode | null>(
    null
  );
  const [focusIn, setFocusIn] = createSignal(false);
  const [enrollmentInProgress, setEnrollmentInProgress] = createSignal(false);
  const [loadingAuthorization, setLoadingAuthorization] = createSignal(false);
  const [expired, setExpired] = createSignal(false);
  const expiresAt = () => loginDeviceCode()?.expiresAt;
  const expiresAtFormat = () => strToMs(expiresAt() || "");
  const expiresAtMs = () => expiresAtFormat() - Date.now();
  const minutes = () => msToMinutes(expiresAtMs());
  const seconds = () => msToSeconds(expiresAtMs());
  const [countDown, setCountDown] = createSignal(
    // eslint-disable-next-line solid/reactivity
    `${minutes()}:${parseTwoDigitNumber(seconds())}`
  );

  let menuRef: undefined | HTMLDivElement;

  const navigate = useGDNavigate();

  const addNotification = createNotification();

  const routeData = useRouteData<typeof fetchData>();

  let interval: ReturnType<typeof setTimeout>;

  const resetCountDown = () => {
    setExpired(false);
    if (!isNaN(minutes()) && !isNaN(seconds())) {
      setCountDown(`${minutes()}:${parseTwoDigitNumber(seconds())}`);
    }
  };

  const updateExpireTime = () => {
    if (minutes() <= 0 && seconds() <= 0) {
      setExpired(true);
    } else {
      resetCountDown();
    }
  };

  createEffect(() => {
    if (expired()) {
      if (enrollmentInProgress()) accountEnrollCancelMutation.mutate(undefined);
      clearInterval(interval);
      if (!isNaN(minutes()) && !isNaN(seconds())) {
        setCountDown(`${minutes()}:${parseTwoDigitNumber(seconds())}`);
      }
    } else {
      interval = setInterval(() => {
        updateExpireTime();
      }, 1000);
    }
  });

  const filteredOptions = () =>
    props.accounts.filter(
      (option) => option.key !== (activeAccount() as Label)?.uuid
    );

  const setActiveUUIDMutation = rspc.createMutation(["account.setActiveUuid"], {
    onMutate: async (
      uuid
    ): Promise<{ previousActiveUUID: string } | undefined> => {
      await queryClient.cancelQueries({ queryKey: ["account.setActiveUuid"] });

      const previousActiveUUID: string | undefined = queryClient.getQueryData([
        "account.setActiveUuid",
      ]);

      queryClient.setQueryData(["account.setActiveUuid", null], uuid);

      if (previousActiveUUID) return { previousActiveUUID };
    },
    onError: (
      error,
      _variables,
      context: { previousActiveUUID: string } | undefined
    ) => {
      addNotification(error.message, "error");

      if (context?.previousActiveUUID) {
        queryClient.setQueryData(
          ["account.setActiveUuid"],
          context.previousActiveUUID
        );
      }
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: ["account.setActiveUuid"] });
    },
  });

  const accountEnrollCancelMutation = rspc.createMutation(
    ["account.enroll.cancel"],
    {
      onError(error) {
        addNotification(error.message, "error");
        setLoginDeviceCode(null);
        setAddAccountStarting(false);
      },
      onMutate() {
        setLoadingAuthorization(false);
        setEnrollmentInProgress(false);
        setLoginDeviceCode(null);
        setAddAccountStarting(false);
      },
    }
  );

  const accountEnrollBeginMutation = rspc.createMutation(
    ["account.enroll.begin"],
    {
      onMutate() {
        setAddAccountStarting(true);
      },
      onError(error) {
        if (enrollmentInProgress())
          accountEnrollCancelMutation.mutate(undefined);
        setAddAccountStarting(false);
        addNotification(error.message, "error");
      },
    }
  );

  const accountEnrollFinalizeMutation = rspc.createMutation(
    ["account.enroll.finalize"],
    {
      onError(error) {
        addNotification(error.message, "error");
      },
      onMutate() {
        setLoadingAuthorization(false);
        setEnrollmentInProgress(false);
      },
    }
  );

  const deleteAccountMutation = rspc.createMutation(["account.deleteAccount"], {
    onMutate: async (
      uuid
    ): Promise<
      | { previousAccounts: AccountEntry[]; previousActiveUUID: string }
      | undefined
    > => {
      await queryClient.cancelQueries({ queryKey: ["account.getAccounts"] });
      await queryClient.cancelQueries({ queryKey: ["account.getActiveUuid"] });

      const previousAccounts: AccountEntry[] | undefined =
        queryClient.getQueryData(["account.getAccounts"]);

      const previousActiveUUID: string | undefined = queryClient.getQueryData([
        "account.getActiveUuid",
      ]);

      queryClient.setQueryData(["account.getActiveUuid", null], null);

      queryClient.setQueryData(
        ["account.getAccounts", null],
        (old: AccountEntry[] | undefined) => {
          const filteredAccounts = old?.filter(
            (account) => account.uuid !== uuid
          );

          if (filteredAccounts) return filteredAccounts;
        }
      );

      if (previousAccounts && previousActiveUUID)
        return { previousAccounts, previousActiveUUID };
    },
    onError: (
      error,
      _variables,
      context:
        | { previousAccounts: AccountEntry[]; previousActiveUUID: string }
        | undefined
    ) => {
      if (routeData.accounts.data?.length === 0) {
        navigate("/");
      } else {
        addNotification(error.message, "error");

        if (context?.previousAccounts) {
          queryClient.setQueryData(
            ["account.getAccounts"],
            context.previousAccounts
          );
        }
        if (context?.previousActiveUUID) {
          queryClient.setQueryData(
            ["account.getActiveUuid"],
            context.previousActiveUUID
          );
        }
      }
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: ["account.getAccounts"] });
    },
    onSuccess() {
      if (routeData.accounts.data?.length === 0) {
        navigate("/");
      }
    },
  });

  const reset = () => {
    setEnrollmentInProgress(false);
    setLoadingAuthorization(false);
    setLoginDeviceCode(null);
    setAddAccountStarting(false);
    setExpired(false);
  };

  createEffect(() => {
    if (routeData.accounts.data?.length === 0) navigate("/");
  });

  createEffect(() => {
    if (routeData.status.isSuccess && !routeData.status.data && expired()) {
      reset();
    }

    handleStatus(routeData.status, {
      onPolling: (info) => {
        setEnrollmentInProgress(true);
        setLoginDeviceCode(info);
      },
      onFail(error) {
        reset();
        accountEnrollCancelMutation.mutate(undefined);
        if (error)
          addNotification(
            "somethign went wrong while adding an account",
            "error"
          );
      },
      onError(error) {
        reset();
        if (error) addNotification(error?.message, "error");
      },
      onComplete() {
        setLoadingAuthorization(false);
        if (enrollmentInProgress()) {
          accountEnrollFinalizeMutation.mutate(undefined);
        }
        reset();
      },
    });
  });

  return (
    <div class="relative inline-block" id={props.id}>
      <p
        class="mt-0 mb-2 font-bold"
        classList={{
          "text-white": !props.disabled,
          "text-darkSlate-50": props.disabled,
        }}
      >
        {props.label}
      </p>
      <button
        class="flex items-center box-border group box-border justify-between py-2 px-4 min-h-10 font-semibold inline-flex rounded-lg w-auto"
        onClick={() => {
          if (props.disabled) return;
          setMenuOpened(!menuOpened());
        }}
        onBlur={() => {
          if (!focusIn()) {
            setMenuOpened(false);
          }
        }}
        classList={{
          "border-0": true,
          "text-darkSlate-50 hover:text-white": !props.disabled,
          rounded: true,
          "bg-darkSlate-700": true,
        }}
      >
        <div class="flex gap-2 items-center">
          <Show when={(activeAccount() as Label)?.icon}>
            <img
              src={(activeAccount() as Label)?.icon}
              class="w-5 h-5 rounded-md"
            />
          </Show>
          <p
            class="m-0 overflow-hidden justify-center w-full text-ellipsis align-middle leading-loose"
            classList={{
              "text-darkSlate-50 hover:text-white group-hover:text-white":
                !props.disabled,
              "text-darkSlate-500": props.disabled,
            }}
          >
            {(activeAccount() as Label)?.name}
          </p>
        </div>

        <span
          class={`i-ri:arrow-drop-up-line text-3xl ease-in-out duration-100 ${
            menuOpened() ? "rotate-180" : "rotate-0"
          }`}
          classList={{
            "text-darkSlate-50 group-hover:text-white": !props.disabled,
            "text-darkSlate-500": props.disabled,
          }}
        />
      </button>
      <div
        ref={menuRef}
        tabindex="0"
        class="rounded-md px-4 w-auto absolute right-0 flex-col text-darkSlate-50 pb-2 mt-1 z-40 min-w-80 pt-3 bg-darkSlate-900"
        onMouseLeave={() => {
          setFocusIn(false);
        }}
        onMouseEnter={() => {
          setFocusIn(true);
        }}
        onClick={() => menuRef?.focus()}
        onBlur={() => {
          setFocusIn(false);
          setMenuOpened(false);
        }}
        classList={{
          flex: menuOpened(),
          hidden: !menuOpened(),
        }}
      >
        <div class="w-full flex flex-col mb-4">
          <div class="flex w-full mb-4">
            <img
              src={(activeAccount() as Label)?.icon}
              class="rounded-md h-10 w-10 mr-2"
            />
            <div class="flex flex-col justify-between">
              <h5 class="m-0 text-white">{(activeAccount() as Label)?.name}</h5>
              <div class="flex gap-1">
                {mapTypeToIcon((activeAccount() as Label)?.type)}
                <p class="m-0 text-xs">{(activeAccount() as Label)?.type}</p>
              </div>
            </div>
          </div>
          <div class="flex items-center gap-3">
            <h5 class="m-0 text-white">
              <Trans
                key="uuid"
                options={{
                  defaultValue: "UUID",
                }}
              />
            </h5>
            <div class="flex items-center gap-1">
              <div class="flex gap-1">
                <p class="m-0 text-xs">{(activeAccount() as Label)?.uuid}</p>
              </div>
              <div
                class="text-darkSlate-50 cursor-pointer transition ease-in-out i-ri:file-copy-fill text-sm hover:text-white"
                onClick={() => {
                  navigator.clipboard.writeText(
                    (activeAccount() as Label)?.uuid
                  );
                  addNotification("The UUID has been copied");
                }}
              />
            </div>
          </div>
        </div>
        <Show when={filteredOptions().length > 0}>
          <hr class="w-full border-darkSlate-50 opacity-20 mb-0" />
        </Show>
        <ul class="text-darkSlate-50 m-0 w-full list-none p-0 shadow-md shadow-darkSlate-900">
          <For each={filteredOptions()}>
            {(option) => {
              return (
                <li class="text-darkSlate-50 flex items-center justify-between min-h-10 no-underline first:rounded-t last:rounded-b block whitespace-no-wrap my-2">
                  <div class="flex gap-2">
                    <img
                      src={(option.label as Label)?.icon}
                      class="w-10 h-10 rounded-md mr-2 grayscale"
                    />
                    <div class="flex flex-col justify-between">
                      <h5 class="m-0 text-white">
                        {(option.label as Label).name}
                      </h5>
                      <div class="m-0">
                        {mapStatus((option.label as Label).status)}
                      </div>
                    </div>
                  </div>

                  <div class="flex gap-3">
                    <div
                      class="cursor-pointer i-ri:delete-bin-7-fill hover:bg-red"
                      onClick={() => {
                        deleteAccountMutation.mutate(
                          (option.label as Label).uuid
                        );
                      }}
                    />
                    <p
                      class="m-0 cursor-pointer hover:text-blue"
                      onClick={() => {
                        setActiveUUIDMutation.mutate(
                          (option.label as Label).uuid
                        );
                      }}
                    >
                      <Trans
                        key="switch_account"
                        options={{
                          defaultValue: "Switch",
                        }}
                      />
                    </p>
                  </div>
                </li>
              );
            }}
          </For>
        </ul>
        <hr class="w-full border-darkSlate-50 opacity-20 mt-0" />
        <div class="flex flex-col">
          <div
            class="flex py-2 justify-between group gap-3"
            classList={{
              "flex-col": !!enrollmentInProgress(),
              "min-h-10": !!enrollmentInProgress(),
              "items-start": !!enrollmentInProgress(),
            }}
          >
            <div class="flex justify-between w-full">
              <div
                class="flex gap-3 items-center"
                classList={{
                  "cursor-not-allowed": !!enrollmentInProgress(),
                  "cursor-pointer": !enrollmentInProgress(),
                }}
              >
                <div
                  class="text-darkSlate-50 transition ease-in-out i-ri:add-circle-fill h-4 w-4"
                  classList={{
                    "text-darkSlate-500": !!enrollmentInProgress(),
                    "group-hover:text-white": !enrollmentInProgress(),
                    "cursor-not-allowed": !!enrollmentInProgress(),
                  }}
                />
                <span
                  class="text-darkSlate-50 transition ease-in-out select-none"
                  classList={{
                    "cursor-not-allowed": !!enrollmentInProgress(),
                  }}
                >
                  <p
                    class="m-0"
                    classList={{
                      "text-darkSlate-500": !!enrollmentInProgress(),
                      "group-hover:text-white": !enrollmentInProgress(),
                    }}
                    onClick={() => {
                      if (!loadingAuthorization()) {
                        if (!enrollmentInProgress()) {
                          accountEnrollBeginMutation.mutate(undefined);
                        } else {
                          accountEnrollCancelMutation.mutate(undefined);
                          accountEnrollBeginMutation.mutate(undefined);
                        }
                        setLoadingAuthorization(true);
                      }
                    }}
                  >
                    <Trans
                      key="add_account"
                      options={{
                        defaultValue: "Add Account",
                      }}
                    />
                  </p>
                </span>
              </div>
              <Show when={addAccountStarting() && !loginDeviceCode()}>
                <Spinner />
              </Show>
            </div>
            <Show when={enrollmentInProgress() && !expired() && expiresAt()}>
              <div class="flex gap-3 items-center justify-between w-full">
                <div class="flex items-center gap-4">
                  <div
                    class="w-5 h-5 rounded-full flex items-center cursor-pointer justify-center"
                    onClick={() => {
                      if (loginDeviceCode()?.verificationUri) {
                        setLoadingAuthorization(true);
                        window.openExternalLink(
                          (loginDeviceCode() as DeviceCode).verificationUri
                        );
                      }
                    }}
                  >
                    <div class="text-sm hover:text-white transition ease-in-out i-ri:external-link-fill" />
                  </div>

                  <div class="flex gap-1 items-center text-xs">
                    <span class="font-bold text-white">
                      {loginDeviceCode()?.userCode}
                    </span>
                    <div
                      class="cursor-pointer text-darkSlate-50 i-ri:file-copy-fill hover:text-white transition ease-in-out"
                      onClick={() => {
                        if (loginDeviceCode()?.userCode) {
                          navigator.clipboard.writeText(
                            (loginDeviceCode() as DeviceCode).userCode
                          );
                        }
                        addNotification("The code has been copied");
                      }}
                    />
                  </div>
                  <div class="text-xs">{countDown()}</div>
                </div>
                <div class="flex gap-3">
                  <Show when={loadingAuthorization()}>
                    <Spinner />
                  </Show>
                  <div
                    class="text-sm cursor-pointer i-ri:close-fill hover:text-red"
                    onClick={() => {
                      if (enrollmentInProgress()) {
                        accountEnrollCancelMutation.mutate(undefined);
                      }
                    }}
                  />
                </div>
              </div>
            </Show>
          </div>
          <div
            class="flex gap-3 py-2 items-center"
            classList={{
              "text-darkSlate-500": !!enrollmentInProgress(),
              "color-red cursor-pointer": !enrollmentInProgress(),
            }}
            onClick={() => {
              if (enrollmentInProgress()) return;
              deleteAccountMutation.mutate((activeAccount() as Label)?.uuid);
            }}
          >
            <div class="h-4 w-4 i-ri:logout-box-fill" />

            <Trans
              key="account_log_out"
              options={{
                defaultValue: "Log out",
              }}
            />
          </div>
        </div>
      </div>
    </div>
  );
};
