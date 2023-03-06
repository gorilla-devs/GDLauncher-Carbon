import { rspc } from "@/utils/rspcClient";
import { Trans } from "@gd/i18n";
import { createSignal, For, Show, JSX, Switch, Match } from "solid-js";

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
  error?: boolean;
  disabled?: boolean;
  rounded?: boolean;
  label?: string;
  onChange?: (_option: Option) => void;
  class?: string;
  id?: string;
  bg?: string;
  icon?: JSX.Element;
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
        <div class="flex gap-2 items-center">
          <div class="w-3 h-3 bg-green rounded-full" />
          <Trans
            key="account_online"
            options={{
              defaultValue: "online",
            }}
          />
        </div>
      }
    >
      <Match when={status === "Ok"}>
        <div class="flex gap-2 items-center">
          <div class="w-3 h-3 bg-green rounded-full" />
          <Trans
            key="account_online"
            options={{
              defaultValue: "online",
            }}
          />
        </div>
      </Match>
      <Match when={status === "Expired"}>
        <div class="flex gap-2 items-center">
          <div class="w-3 h-3 bg-red rounded-full" />
          <Trans
            key="account_expired"
            options={{
              defaultValue: "Expired",
            }}
          />
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
  const [focusIn, setFocusIn] = createSignal(false);

  const toggleMenu = () => {
    if (props.disabled) return;
    setMenuOpened(true);
    setTimeout(() => {
      setMenuOpened(false);
    }, 100);
  };

  return (
    <div class="inline-block relative" id={props.id}>
      <Show when={!props.rounded && props.label}>
        <p
          class="mt-0 mb-2 font-bold"
          classList={{
            "text-white": !props.disabled,
            "text-shade-0": props.disabled,
          }}
        >
          {props.label}
        </p>
      </Show>
      <button
        class={`group flex justify-between font-semibold py-2 px-4 inline-flex items-center min-h-10 box-border ${props.class} ${props.bg}`}
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
          "border-0": !props.error,
          "border-1 border-status-red": props.error,
          "text-shade-0 hover:text-white": !props.disabled && !props.error,
          "text-shade-5": props.error,
          "rounded-full": props.rounded,
          rounded: !props.rounded,
          "bg-shade-7": !props.bg,
        }}
      >
        <Show when={props.icon}>
          <span class="mr-2">{props.icon}</span>
        </Show>
        <Show when={(selectedValue() as Label).icon}>
          <img
            src={(selectedValue() as Label).icon}
            class="w-5 h-5 rounded-md mr-2"
          />
        </Show>
        <span
          class="w-full"
          classList={{
            "text-white": props.error,
            "text-shade-0 hover:text-white group-hover:text-white":
              !props.disabled && !props.error,
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
            "text-shade-0 group-hover:text-white":
              !props.disabled && !props.error,
            "text-white": props.error,
            "text-shade-5": props.disabled,
          }}
        />
      </button>
      <div
        class="absolute flex-col text-shade-0 bg-shade-9 py-2 px-4 right-0 rounded-md mt-1 w-auto z-40 min-w-80"
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
              class="w-10 h-10 rounded-md mr-2"
            />
            <div class="flex flex-col justify-between">
              <h5 class="m-0 text-white">{(selectedValue() as Label).name}</h5>
              <p class="m-0 text-xs">{(selectedValue() as Label).type}</p>
            </div>
          </div>
          <h5 class="mt-0 mb-2 text-white">
            <Trans
              key="uuid"
              options={{
                defaultValue: "UUID",
              }}
            />
          </h5>
          <p class="m-0 text-xs">{(selectedValue() as Label).uuid}</p>
        </div>
        <hr class="w-full border-shade-0 opacity-20 mb-0" />
        <ul class="text-shade-0 shadow-md shadow-shade-9 list-none m-0 p-0 w-full">
          <For
            each={props.options}
            // each={props.options.filter(
            //   (option) => option.key !== (selectedValue() as Label).uuid
            // )}
          >
            {(option) => {
              let accountStatusQuery = rspc.createQuery(() => [
                "account.getAccountStatus",
                (selectedValue() as Label).uuid,
              ]);

              return (
                <li
                  class="first:rounded-t last:rounded-b block whitespace-no-wrap text-shade-0 no-underline min-h-10 my-2 flex items-center justify-between hover:bg-shade-8 pr-2"
                  onClick={() => {
                    setSelectedValue(option.label);
                    props.onChange?.(option);
                    toggleMenu();
                  }}
                >
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

                  <p class="m-0 hover:text-blue">
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
          <div class="group flex gap-3 py-2 items-center cursor-pointer">
            <div class="i-ri:add-circle-fill h-4 w-4 text-shade-0 group-hover:text-white transition ease-in-out" />
            <span class="text-shade-0 group-hover:text-white transition ease-in-out">
              <Trans
                key="add_account"
                options={{
                  defaultValue: "Add Account",
                }}
              />
            </span>
          </div>
          <div class="flex gap-3 py-2 items-center color-red cursor-pointer">
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
