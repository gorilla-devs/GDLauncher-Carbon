import Sidebar from "@/components/Sidebar/library";
import { Outlet } from "@solidjs/router";
import ContentWrapper from "@/components/ContentWrapper";
import { Show, onMount } from "solid-js";
import { rspc } from "@/utils/rspcClient";
import { useModal } from "@/managers/ModalsManager";

function Library() {
  const gridLayout = () => false;
  const modalsManager = useModal();
  const settings = rspc.createQuery(() => ({
    queryKey: ["settings.getSettings"]
  }));
  const updateSettings = rspc.createMutation(() => ({
    mutationKey: ["settings.setSettings"]
  }));

  onMount(() => {
    console.log(settings.data?.lastAppVersion, __APP_VERSION__);
    if (settings.data?.lastAppVersion !== __APP_VERSION__) {
      modalsManager?.openModal({
        name: "changelogs"
      });

      updateSettings.mutate({
        lastAppVersion: {
          Set: __APP_VERSION__
        }
      });
    }
  });

  return (
    <>
      <Show when={gridLayout()}>
        <Sidebar />
      </Show>
      <ContentWrapper zeroPadding>
        <Outlet />
      </ContentWrapper>
    </>
  );
}

export default Library;
