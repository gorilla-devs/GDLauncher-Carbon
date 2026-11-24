import { createContext, createSignal, useContext } from "solid-js"

/** Where the advisory toast's "Review" lands: every Java with the advisory
 *  stays highlighted until its row is hovered or focused, and the first one
 *  is scrolled to, with its warning icon focused and its popover open. */
export type AdvisoryReview = ReturnType<typeof createAdvisoryReview>

export function createAdvisoryReview() {
  const [highlighted, setHighlighted] = createSignal<ReadonlySet<string>>(
    new Set()
  )
  const [landing, setLanding] = createSignal<{
    javaId: string
    scroll: ScrollBehavior
  }>()

  return {
    /** `javaIds` in the list's order. */
    start(javaIds: string[], scroll: ScrollBehavior) {
      setHighlighted(new Set(javaIds))
      setLanding(
        javaIds.length > 0 ? { javaId: javaIds[0], scroll } : undefined
      )
    },
    isHighlighted: (javaId: string) => highlighted().has(javaId),
    unhighlight(javaId: string) {
      setHighlighted((ids) => {
        if (!ids.has(javaId)) return ids
        const next = new Set(ids)
        next.delete(javaId)
        return next
      })
    },
    /** The row to land on, until it calls `landed`. */
    landing,
    landed: () => setLanding(undefined)
  }
}

export const AdvisoryReviewContext = createContext<AdvisoryReview>()

export const useAdvisoryReview = () => useContext(AdvisoryReviewContext)
