import { rspc } from "@/utils/rspcClient";
import { Input, Tooltip } from "@gd/ui";
import { Match, Switch, createEffect, createSignal } from "solid-js";
import TruncatedPath from "./TruncatePath";

type Props = {
  updateValue?: (_value: string) => void;
  disabled?: boolean;
};

const JavaPathAutoComplete = (props: Props) => {
  const [value, setValue] = createSignal("");
  let availableJavas = rspc.createQuery(() => ["java.getAvailableJavas"]);

  const createCustomJavaVersionMutation = rspc.createMutation([
    "java.createCustomJavaVersion"
  ]);

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
                    <div>Not found</div>
                    <div>Create new [CUSTOM]</div>
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
  };

  createEffect(() => {
    if (javaComponent()?.id) {
      props.updateValue?.(javaComponent()?.id!);
    } else {
      props.updateValue?.("");
    }
  });

  return (
    <div>
      <Input
        value={value()}
        disabled={props.disabled}
        placeholder="Type a java path"
        inputColor="bg-darkSlate-600"
        icon={
          <Switch>
            <Match when={createCustomJavaVersionMutation.isLoading}>
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
        autoCompleteOptions={autocompleteOptions()}
      />
    </div>
  );
};

export default JavaPathAutoComplete;
