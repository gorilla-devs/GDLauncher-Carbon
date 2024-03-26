import { Trans } from "@gd/i18n";
import { ModalProps, useModal } from "@/managers/ModalsManager";
import ModalLayout from "@/managers/ModalsManager/ModalLayout";
import { Button, Input, createNotification } from "@gd/ui";
import { rspc } from "@/utils/rspcClient";
import { Match, Switch, createSignal } from "solid-js";

const AddCustomJava = (props: ModalProps) => {
  const [value, setValue] = createSignal("");
  const [validPath, setValidPath] = createSignal<boolean | null>(null);
  const modalsContext = useModal();
  const addNotification = createNotification();

  const createCustomJavaVersionMutation = rspc.createMutation(() => ({
    mutationKey: ["java.createCustomJavaVersion"]
  }));

  const validateJavaPath = rspc.createMutation(() => ({
    mutationKey: ["java.validateCustomJavaPath"]
  }));

  return (
    <ModalLayout width="w-100" noHeader={props.noHeader} title={props?.title}>
      <div class="flex items-center h-full flex-col justify-center w-full">
        <div class="flex flex-col gap-8 w-full">
          <div class="flex justify-between items-center gap-4 w-full">
            <Input
              class="w-full"
              value={value()}
              inputColor="bg-darkSlate-600"
              placeholder="Type a custom java path"
              icon={
                <Switch>
                  <Match when={validateJavaPath.isPending}>
                    <div class="flex i-ri:loader-4-line animate-spin text-darkSlate-50" />
                  </Match>
                  <Match when={value()}>
                    <Switch>
                      <Match when={validPath() === true}>
                        <div class="flex i-ri:checkbox-circle-fill text-emerald-500" />
                      </Match>
                      <Match when={validPath() === false}>
                        <div class="flex i-ri:error-warning-fill text-yellow-500" />
                      </Match>
                    </Switch>
                  </Match>
                  <Match when={true}>
                    <div />
                  </Match>
                </Switch>
              }
              onInput={(e) => setValue(e.currentTarget.value)}
              onBlur={async () => {
                if (!value()) {
                  setValidPath(null);
                  return;
                }
                const res = await validateJavaPath.mutateAsync(value());
                setValidPath(res);
              }}
            />
          </div>
          <div class="flex w-full justify-end">
            <Button
              disabled={
                !validPath() ||
                validateJavaPath.isPending ||
                createCustomJavaVersionMutation.isPending
              }
              onClick={() => {
                createCustomJavaVersionMutation.mutate(value());
                addNotification(
                  "Custom Java Path added successfully",
                  "success"
                );
                modalsContext?.closeModal();
              }}
            >
              <Trans key="add_custom_java_path" />
            </Button>
          </div>
        </div>
      </div>
    </ModalLayout>
  );
};

export default AddCustomJava;
