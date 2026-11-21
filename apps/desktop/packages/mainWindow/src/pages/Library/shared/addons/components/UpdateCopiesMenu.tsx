import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger
} from "@gd/ui"
import { Trans } from "@gd/i18n"
import { JSX } from "solid-js"

/** The update arrow of a data pack with several copies in the instance:
 *  update this copy, or every copy with an update. */
export const UpdateCopiesMenu = (props: {
  /** The arrow the menu opens from. */
  children: JSX.Element
  copies: number
  testId: string
  onUpdateHere: () => void
  onUpdateEverywhere: () => void
}) => (
  <DropdownMenu placement="bottom-start">
    <DropdownMenuTrigger class="b-0 bg-transparent p-0">
      {props.children}
    </DropdownMenuTrigger>
    <DropdownMenuContent>
      <DropdownMenuItem
        data-testid={`${props.testId}-here`}
        onSelect={() => props.onUpdateHere()}
      >
        <Trans key="content:_trn_worlds.update_here" />
      </DropdownMenuItem>
      <DropdownMenuItem
        data-testid={`${props.testId}-everywhere`}
        onSelect={() => props.onUpdateEverywhere()}
      >
        <div class="flex flex-1 items-center justify-between gap-4">
          <Trans key="content:_trn_worlds.update_everywhere" />
          <span class="text-lightSlate-600">{props.copies}</span>
        </div>
      </DropdownMenuItem>
    </DropdownMenuContent>
  </DropdownMenu>
)
