import { rspc } from "@/utils/rspcClient";
import { Input, Tooltip } from "@gd/ui";
import {
  Match,
  Switch,
  createEffect,
  createSignal,
  getOwner,
  runWithOwner
} from "solid-js";
import TruncatedPath from "./TruncatePath";
import { Trans } from "@gd/i18n";

type Props = {
  defaultValue?: string;
  updateValue?: (_id: string | null, _value: string) => void;
  updateValueOnlyOnBlur?: boolean;
  disabled?: boolean;
  inputColor?: string;
};

const JavaPathAutoComplete = (props: Props) => {
  const owner = getOwner();
  const [value, setValue] = createSignal(props.defaultValue || "");
  let availableJavas = rspc.createQuery(() => ({
    queryKey: ["java.getAvailableJavas"]
  }));

  let runOnce = false;

  createEffect(() => {
    if (!runOnce && !value() && props.defaultValue) {
      setValue(props.defaultValue);
      runOnce = true;
    }
  });

  const createCustomJavaVersionMutation = rspc.createMutation(() => ({
    mutationKey: "java.createCustomJavaVersion"
  }));

  const _autocompleteOptions = () => {
    if (!availableJavas.data) return [];

    const values = Object.values(availableJavas.data)
      .flat()
      .filter(
        (java) =>
          java.path.toLowerCase().includes(value().toLowerCase()) ||
          java.version.toLowerCase().includes(value().toLowerCase()) ||
          java.type.toLowerCase().includes(value().toLowerCase())
      )
      .map((java) => ({
        value: java.path,
        label: (
          <div class="w-full flex flex-col gap-2">
            <div class="flex justify-between">
              <div class="text-white">{java.version}</div>
              <div>{java.type}</div>
            </div>
            <Tooltip content={java.path}>
              <TruncatedPath originalPath={java.path} />
            </Tooltip>
          </div>
        )
      }));

    if (values.length === 1 && values[0].value === value()) {
      return [];
    }

    return values;
  };

  const javaComponent = () => {
    return Object.values(availableJavas.data || {})
      ?.flat()
      .find((java) => java.path === value());
  };

  const shouldSuggestCreation = () => {
    return !javaComponent() && value() && _autocompleteOptions().length === 0;
  };

  const autocompleteOptions = () => {
    const x = runWithOwner(owner, () => {
      return _autocompleteOptions().concat(
        shouldSuggestCreation()
          ? [
              {
                value: value(),
                label: (
                  <div
                    class="w-full flex flex-col gap-2"
                    onClick={() => {
                      createCustomJavaVersionMutation.mutate(value());
                    }}
                  >
                    <div class="flex justify-between">
                      <div>
                        <Trans key="java_autocomplete.not_found" />
                      </div>
                      <div>
                        <Trans key="java_autocomplete.create_new_custom" />
                      </div>
                    </div>
                    <Tooltip content={value()}>
                      <TruncatedPath originalPath={value()} />
                    </Tooltip>
                  </div>
                )
              }
            ]
          : []
      );
    });

    return x;
  };

  createEffect(() => {
    if (!props.updateValueOnlyOnBlur) {
      props.updateValue?.(javaComponent()?.id || null, value());
    }
  });

  return (
    <div>
      <Input
        value={value()}
        disabled={props.disabled}
        placeholder="Type a java path"
        inputColor={props.inputColor || ""}
        icon={
          <Switch>
            <Match when={createCustomJavaVersionMutation.isPending}>
              <div class="flex i-ri:loader-4-line animate-spin text-darkSlate-50" />
            </Match>
            <Match when={javaComponent()}>
              <Switch>
                <Match when={javaComponent()?.isValid}>
                  <div class="flex i-ri:checkbox-circle-fill text-emerald-500" />
                </Match>
                <Match when={!javaComponent()?.isValid}>
                  <div class="flex i-ri:error-warning-fill text-yellow-500" />
                </Match>
              </Switch>
            </Match>
            <Match when={shouldSuggestCreation()}>
              <div class="flex i-ri:add-fill text-darkSlate-50" />
            </Match>
          </Switch>
        }
        onSearch={(value) => {
          setValue(value);
        }}
        onBlur={() => {
          if (props.updateValueOnlyOnBlur) {
            props.updateValue?.(javaComponent()?.id || null, value());
          }
        }}
        autoCompleteOptions={autocompleteOptions() || []}
      />
    </div>
  );
};

export default JavaPathAutoComplete;
