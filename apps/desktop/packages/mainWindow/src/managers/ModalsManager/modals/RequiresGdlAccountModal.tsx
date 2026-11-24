import { useGDNavigate } from "@/managers/NavigationManager"
import { Trans } from "@gd/i18n"
import { Button } from "@gd/ui"
import ModalLayout from "../ModalLayout"
import { CancelLink, DialogFooter } from "../DialogFooter"
import { ModalProps, useModal } from ".."

interface Data {
  returnPath?: string
}

/**
 * The path without the `m[n]` entries that name the open modals: all of them
 * are closed on the way to the sign-in page, so none is open on the return.
 */
function withoutModalParams(path: string): string {
  const [pathname, search = ""] = path.split("?")
  const params = new URLSearchParams(search)
  const modalKeys = [...params.keys()].filter((key) => /^m\[\d+\]$/.test(key))
  if (modalKeys.length === 0) return path
  for (const key of modalKeys) params.delete(key)
  const rest = params.toString()
  return rest ? `${pathname}?${rest}` : pathname
}

function RequiresGdlAccountModal(props: ModalProps) {
  const navigator = useGDNavigate()
  const modalsContext = useModal()

  const data = () => props.data as Data | undefined

  const handleLinkAccount = () => {
    const returnTo = withoutModalParams(data()?.returnPath || "/")
    navigator.navigate(
      `/?addGdlAccount=true&returnTo=${encodeURIComponent(returnTo)}`
    )
    // The page under the dialogs is being left for the sign-in page, so a
    // dialog this one was opened from must not stay open over it.
    modalsContext?.closeAllModals()
  }

  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title}>
      <div class="w-100 flex min-h-40 flex-col justify-between gap-6">
        <div class="flex flex-col gap-4">
          <div class="text-lightSlate-200 text-sm">
            <Trans key="accounts:_trn_requires_gdl_account_modal_description" />
          </div>
        </div>

        <DialogFooter>
          <CancelLink onClick={() => modalsContext?.closeModal()} />
          <Button type="primary" size="large" onClick={handleLinkAccount}>
            <Trans key="accounts:_trn_link_account" />
          </Button>
        </DialogFooter>
      </div>
    </ModalLayout>
  )
}

export default RequiresGdlAccountModal
