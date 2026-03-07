import ContentWrapper from "@/components/ContentWrapper"
import { createEffect, createSignal, JSX } from "solid-js"
import { rspc } from "@/utils/rspcClient"
import { useModal } from "@/managers/ModalsManager"

function Library(props: { children?: JSX.Element }) {
  const modalsManager = useModal()
  const [changelogShown, setChangelogShown] = createSignal(false)

  const shouldShowChangelog = rspc.createQuery(() => ({
    queryKey: ["settings.shouldShowChangelog"]
  }))

  const markChangelogSeen = rspc.createMutation(() => ({
    mutationKey: ["settings.markChangelogSeen"]
  }))

  createEffect(() => {
    if (shouldShowChangelog.data === true && !changelogShown()) {
      setChangelogShown(true)
      modalsManager?.openModal({
        name: "changelogs"
      })
      markChangelogSeen.mutate(undefined)
    }
  })

  return (
    <>
      <ContentWrapper zeroPadding>
        {props.children}
      </ContentWrapper>
    </>
  )
}

export default Library
