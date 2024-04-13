import { onCleanup, onMount } from "solid-js";
import { ModalProps, useModal } from "..";
import ModalLayout from "../ModalLayout";
import { Trans } from "@gd/i18n";
import { Button, Checkbox } from "@gd/ui";
import { rspc } from "@/utils/rspcClient";
import { useGDNavigate } from "@/managers/NavigationManager";

export let windowCloseWarningAcquireLock = true;

const WindowCloseWarning = (props: ModalProps) => {
  const navigate = useGDNavigate();
  const modalsManager = useModal();

  const settings = rspc.createQuery(() => ({
    queryKey: ["settings.getSettings"]
  }));

  const settingsMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.setSettings"]
  }));

  onMount(() => {
    windowCloseWarningAcquireLock = false;
  });

  onCleanup(() => {
    windowCloseWarningAcquireLock = true;
  });

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props.title}
      height="h-70"
      width="w-140"
    >
      <div class="flex flex-col justify-between h-full overflow-y-auto text-lightSlate-300">
        <div class="flex flex-col gap-8">
          <div class="text-yellow-400 text-center font-bold text-xl">
            <Trans key="window_close_title" />
          </div>
          <div class="flex flex-col gap-4">
            <div>
              <Trans key="window_close_text_1" />
            </div>
            <div>
              <Trans key="window_close_text_2">
                {""}
                <span
                  class="text-lightSlate-300 underline hover:text-lightSlate-100 transition-colors ease-in-out duration-100"
                  onClick={() => {
                    navigate("/settings");
                    modalsManager?.closeModal();
                    setTimeout(() => {
                      document
                        .getElementById("launcher_action_on_game_launch")
                        ?.scrollIntoView({
                          behavior: "smooth",
                          block: "center"
                        });
                    }, 150);
                  }}
                />
                {""}
              </Trans>
            </div>
          </div>
        </div>

        <div class="flex justify-between items-center">
          <Checkbox
            checked={!settings.data?.showAppCloseWarning}
            onChange={(checked) => {
              settingsMutation.mutate({
                showAppCloseWarning: {
                  Set: !checked
                }
              });
            }}
          >
            <Trans key="window_close_never_show" />
          </Checkbox>
          <Button
            type="secondary"
            class="w-full"
            onClick={() => {
              window.closeWindow();
            }}
          >
            <Trans key="window_close_quit_app" />
          </Button>
        </div>
      </div>
    </ModalLayout>
  );
};

export default WindowCloseWarning;
