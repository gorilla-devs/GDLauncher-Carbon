import RowContainer, { Props } from "@/components/Browser/RowContainer"
import { createEffect, createSignal } from "solid-js"

const VersionRow = (props: Props) => {
  const [loading, setLoading] = createSignal(false)

  const isInstalled = () => {
    return (
      props.installedFile?.remoteId.toString() ===
        props.modVersion?.fileId.toString() &&
      props.installedFile?.remoteId !== null
    )
  }

  createEffect(() => {
    if (isInstalled()) {
      setLoading(false)
    }
  })

  return (
    <RowContainer
      {...props}
      loading={loading()}
      disabled={!props.instanceId}
      isInstalled={isInstalled()}
    />
  )
}

export default VersionRow
