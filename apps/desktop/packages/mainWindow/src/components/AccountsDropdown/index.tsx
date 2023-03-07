import { parseTwoDigitNumber } from "@/utils/helpers";
import { handleStatus } from "@/utils/login";
import { rspc } from "@/utils/rspcClient";
import { DeviceCode } from "@gd/core_module/bindings";
import { Trans } from "@gd/i18n";
import { createNotification } from "@gd/ui";
import {
  createSignal,
  For,
  Show,
  JSX,
  Switch,
  Match,
  createEffect,
} from "solid-js";

export type Label = {
  name: string;
  uuid: string;
  type: string;
  icon: string;
};

export type Option = {
  label: Label;
  key: string;
};

export type OptionDropDown = {
  label: string;
  key: string;
};

export type Props = {
  options: Option[];
  value: string;
  disabled?: boolean;
  label?: string;
  id?: string;
};
export interface DropDownButtonProps extends Props {
  children: JSX.Element;
}

const parseStatus = (
  status: "Ok" | "Expired" | "Refreshing" | null | undefined
) => {
  return (
    <Switch
      fallback={
        <div class="flex items-center gap-2">
          <div class="w-3 h-3 bg-green rounded-full" />
          <p class="m-0 text-xs">
            <Trans
              key="account_online"
              options={{
                defaultValue: "online",
              }}
            />
          </p>
        </div>
      }
    >
      <Match when={status === "Ok"}>
        <div class="flex gap-2 items-center">
          <div class="w-3 h-3 bg-green rounded-full" />
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
      <Match when={status === "Expired"}>
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
      <Match when={status === "Refreshing"}>
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

export const AccountsDropdown = (props: Props) => {
  const defaultValue = () =>
    props.options.find((option) => option.key === props.value)?.label ||
    props.options[0]?.label;

  const [selectedValue, setSelectedValue] = createSignal(defaultValue());
  const [menuOpened, setMenuOpened] = createSignal(false);
  const [loginDeviceCode, setLoginDeviceCode] = createSignal<DeviceCode>({});
  const [focusIn, setFocusIn] = createSignal(false);
  const [expired, setExpired] = createSignal(false);
  const [addCompleted, setAddCompleted] = createSignal(true);

  const expiresAt = () => loginDeviceCode().expires_at;
  const expiresAtFormat = () => new Date(expiresAt())?.getTime();
  const expiresAtMs = () => expiresAtFormat() - Date.now();
  const minutes = () =>
    Math.floor((expiresAtMs() % (1000 * 60 * 60)) / (1000 * 60));
  const seconds = () => Math.floor((expiresAtMs() % (1000 * 60)) / 1000);
  const [countDown, setCountDown] = createSignal(
    // eslint-disable-next-line solid/reactivity
    `${minutes()}:${parseTwoDigitNumber(seconds())}`
  );

  const [addNotification] = createNotification();

  let interval: ReturnType<typeof setTimeout>;

  const resetCountDown = () => {
    setExpired(false);
    setCountDown(`${minutes()}:${parseTwoDigitNumber(seconds())}`);
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
      clearInterval(interval);
      setCountDown(`${minutes()}:${parseTwoDigitNumber(seconds())}`);
    } else {
      interval = setInterval(() => {
        updateExpireTime();
      }, 1000);
    }
  });

  const filteredOptions = () =>
    props.options.filter(
      (option) => option.key !== (selectedValue() as Label).uuid
    );

  const setActiveUUIDMutation = rspc.createMutation(["account.setActiveUuid"], {
    onMutate: (uuid) => {
      const selectedAccount = props.options.find(
        (account) => account.label.uuid === uuid
      );

      if (selectedAccount) {
        setSelectedValue(selectedAccount.label);
      }
    },
  });

  const accountEnrollBeginMutation = rspc.createMutation(
    ["account.enroll.begin"],
    {
      onError() {
        accountEnrollCancelMutation.mutate(null);
        accountEnrollBeginMutation.mutate(null);
      },
    }
  );

  const accountEnrollCancelMutation = rspc.createMutation([
    "account.enroll.cancel",
  ]);

  const accountEnrollFinalizeMutation = rspc.createMutation([
    "account.enroll.finalize",
  ]);

  const data = rspc.createQuery(() => ["account.enroll.getStatus", null]);

  createEffect(() => {
    if (data.isSuccess) {
      handleStatus(data, {
        onPolling: (info) => {
          setAddCompleted(false);
          setLoginDeviceCode(info);
        },
        onFail() {
          setAddCompleted(true);
          accountEnrollCancelMutation.mutate(null);
          accountEnrollBeginMutation.mutate(null);
        },
        onComplete() {
          accountEnrollFinalizeMutation.mutate(null);
          setAddCompleted(true);
        },
      });
    }
  });

  return (
    <div class="relative inline-block" id={props.id}>
      <p
        class="mt-0 mb-2 font-bold"
        classList={{
          "text-white": !props.disabled,
          "text-shade-0": props.disabled,
        }}
      >
        {props.label}
      </p>
      <button
        class="group flex justify-between py-2 px-4 items-center min-h-10 box-border font-semibold inline-flex rounded-lg"
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
          "text-shade-0 hover:text-white": !props.disabled,
          rounded: true,
          "bg-shade-7": true,
        }}
      >
        <Show when={(selectedValue() as Label).icon}>
          <img
            src={(selectedValue() as Label).icon}
            class="mr-2 w-5 h-5 rounded-md"
          />
        </Show>
        <span
          class="w-full"
          classList={{
            "text-shade-0 hover:text-white group-hover:text-white":
              !props.disabled,
            "text-shade-5": props.disabled,
          }}
        >
          {(selectedValue() as Label).name}
        </span>

        <span
          class={`i-ri:arrow-drop-up-line text-3xl ease-in-out duration-100 ${
            menuOpened() ? "rotate-180" : "rotate-0"
          }`}
          classList={{
            "text-shade-0 group-hover:text-white": !props.disabled,
            "text-shade-5": props.disabled,
          }}
        />
      </button>
      <div
        class="absolute right-0 rounded-md flex-col text-shade-0 bg-shade-9 py-2 px-4 mt-1 w-auto z-40 min-w-80"
        onMouseOut={() => {
          setFocusIn(false);
        }}
        onMouseOver={() => {
          setFocusIn(true);
        }}
        classList={{
          flex: menuOpened(),
          hidden: !menuOpened(),
        }}
      >
        <div class="w-full flex flex-col mb-4">
          <div class="flex w-full mb-6">
            <img
              src={(selectedValue() as Label).icon}
              class="h-10 rounded-md mr-2 w-10"
            />
            <div class="flex flex-col justify-between">
              <h5 class="m-0 text-white">{(selectedValue() as Label).name}</h5>
              <p class="m-0 text-xs">{(selectedValue() as Label).type}</p>
            </div>
          </div>
          <div class="flex gap-3">
            <h5 class="mt-0 mb-2 text-white">
              <Trans
                key="uuid"
                options={{
                  defaultValue: "UUID",
                }}
              />
            </h5>
            <div class="flex gap-1">
              <p class="m-0 text-xs">{(selectedValue() as Label).uuid}</p>
              <div
                class="cursor-pointer text-shade-0 i-ri:file-copy-fill text-sm hover:text-white transition ease-in-out"
                onClick={() => {
                  navigator.clipboard.writeText(
                    (selectedValue() as Label).uuid
                  );
                  addNotification("The UUID has been copied");
                }}
              />
            </div>
          </div>
        </div>
        <Show when={filteredOptions().length > 0}>
          <hr class="w-full border-shade-0 opacity-20 mb-0" />
        </Show>
        <ul class="text-shade-0 m-0 w-full shadow-md shadow-shade-9 list-none p-0">
          <For each={filteredOptions()}>
            {(option) => {
              // eslint-disable-next-line solid/reactivity
              const accountStatusQuery = rspc.createQuery(() => [
                "account.getAccountStatus",
                (selectedValue() as Label).uuid,
              ]);

              return (
                <li class="text-shade-0 flex items-center justify-between first:rounded-t last:rounded-b block whitespace-no-wrap no-underline min-h-10 my-2">
                  <div class="flex gap-2">
                    <img
                      src={(option.label as Label).icon}
                      class="w-10 h-10 rounded-md mr-2"
                    />
                    <div class="flex flex-col">
                      <h5 class="m-0 text-white">
                        {(option.label as Label).name}
                      </h5>
                      <p class="m-0">{parseStatus(accountStatusQuery.data)}</p>
                    </div>
                  </div>

                  <p
                    class="m-0 hover:text-blue cursor-pointer"
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
                </li>
              );
            }}
          </For>
        </ul>
        <hr class="w-full border-shade-0 opacity-20 mt-0" />
        <div class="flex flex-col">
          <div class="flex py-2 items-center justify-between cursor-pointer group gap-3">
            <div class="flex gap-3">
              <div class="text-shade-0 i-ri:add-circle-fill h-4 w-4 group-hover:text-white transition ease-in-out" />
              <span class="text-shade-0 group-hover:text-white transition ease-in-out">
                <p
                  class="m-0"
                  onClick={() => {
                    accountEnrollBeginMutation.mutate(null);
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
            <Show when={!addCompleted()}>
              <div class="flex gap-3 items-center">
                <div
                  class="w-5 h-5 bg-blue rounded-full flex justify-center items-center"
                  onClick={() => {
                    if (loginDeviceCode().verification_uri)
                      window.openExternalLink(
                        loginDeviceCode().verification_uri
                      );
                  }}
                >
                  <div class="i-ri:link text-md text-white" />
                </div>

                <div class="flex gap-1 items-center text-xs">
                  {loginDeviceCode().user_code}
                  <div
                    class="cursor-pointer text-shade-0 i-ri:file-copy-fill hover:text-white transition ease-in-out"
                    onClick={() => {
                      navigator.clipboard.writeText(
                        loginDeviceCode().user_code
                      );
                      addNotification("The code has been copied");
                    }}
                  />
                </div>
                <div class="text-xs">{countDown()}</div>
              </div>
            </Show>
          </div>
          <div class="flex gap-3 py-2 items-center cursor-pointer color-red">
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
