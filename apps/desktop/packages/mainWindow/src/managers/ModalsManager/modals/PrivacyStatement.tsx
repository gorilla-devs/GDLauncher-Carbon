/* eslint-disable solid/no-innerhtml */
import { rspc } from "@/utils/rspcClient"
import sanitizeHtml from "sanitize-html"
import { ModalProps } from ".."
import ModalLayout from "../ModalLayout"
import { Suspense } from "solid-js"

const PrivacyStatement = (props: ModalProps) => {
  const body = rspc.createQuery(() => ({
    queryKey: ["settings.getPrivacyStatementBody"]
  }))

  const sanitizedHtml = () => {
    if (!body.data) return undefined
    return sanitizeHtml(body.data)
  }

  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title}>
      <Suspense>
        <div class="h-130 w-190 overflow-hidden">
          {/* Tab reaches the text, so the arrow keys scroll it. */}
          <div
            tabindex="0"
            role="region"
            aria-label={props.title}
            class="max-h-full overflow-y-auto outline-none focus-visible:outline focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-primary-500"
            innerHTML={sanitizedHtml()}
          />
        </div>
      </Suspense>
    </ModalLayout>
  )
}

export default PrivacyStatement
