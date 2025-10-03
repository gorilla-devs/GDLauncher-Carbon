import { Outlet } from "@solidjs/router"
import ContentWrapper from "@/components/ContentWrapper"
import { onMount } from "solid-js"
import { rspc } from "@/utils/rspcClient"
import { useModal } from "@/managers/ModalsManager"

function Library() {
  const modalsManager = useModal()
  const settings = rspc.createQuery(() => ({
    queryKey: ["settings.getSettings"]
  }))
  const updateSettings = rspc.createMutation(() => ({
    mutationKey: ["settings.setSettings"]
  }))

  onMount(() => {
    if (settings.data?.lastAppVersion !== __APP_VERSION__) {
      modalsManager?.openModal({
        name: "changelogs"
      })

      updateSettings.mutate({
        lastAppVersion: {
          Set: __APP_VERSION__
        }
      })
    }
  })

  return (
    <>
      <ContentWrapper zeroPadding>
        <Outlet />
      </ContentWrapper>
    </>
  )
}

export default Library
