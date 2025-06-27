import { Button } from "@gd/ui"
import { Trans } from "@gd/i18n"
import skull from "/assets/images/icons/skull.png"

interface NoAddonsProps {
  onAddAddons: () => void
}

export const NoAddons = (props: NoAddonsProps) => {
  return (
    <div class="min-h-90 flex h-full w-full items-center justify-center">
      <div class="flex flex-col items-center justify-center text-center">
        <img src={skull} class="h-16 w-16" />
        <p class="text-lightSlate-700 max-w-100">
          <Trans key="instance.no_addons_text" />
        </p>
        <Button type="outline" size="medium" onClick={props.onAddAddons}>
          <Trans key="instance.add_addons" />
        </Button>
      </div>
    </div>
  )
}
