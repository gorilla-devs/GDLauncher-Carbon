import { Match, Show, Switch } from "solid-js"
import forgeIcon from "/assets/images/icons/forge.png"
import fabricIcon from "/assets/images/icons/fabric.png"
import quiltIcon from "/assets/images/icons/quilt.svg"
import vanillaIcon from "/assets/images/icons/vanilla.png"
import { FEUnifiedModLoaderType } from "@gd/core_module/bindings"

export const getModloaderIcon = (modloader: string) => {
  switch (modloader?.toString().toLowerCase()) {
    case "forge":
      return forgeIcon
    case "neoforge":
      return forgeIcon // NeoForge uses Forge icon as fallback
    case "fabric":
      return fabricIcon
    case "quilt":
      return quiltIcon
    default:
      return vanillaIcon
  }
}

export const ModloaderIcon = (props: { modloader: FEUnifiedModLoaderType }) => {
  return (
    <Switch
      fallback={
        <>
          <Show when={getModloaderIcon(props.modloader)}>
            <div
              class="h-4 w-4"
              // eslint-disable-next-line solid/no-innerhtml
              innerHTML={getModloaderIcon(props.modloader)}
            />
          </Show>
        </>
      }
    >
      <Match when={typeof props.modloader === "string"}>
        <img class="h-4 w-4" src={getModloaderIcon(props.modloader)} />
      </Match>
    </Switch>
  )
}
