import { createEffect, Accessor } from "solid-js"
import { useGDNavigate } from "@/managers/NavigationManager"

interface UseEntityGoneGuardOptions<T> {
  /** Route id to look for in `list`. */
  id: Accessor<number>
  /** Last-known list the entity `id` refers to should appear in. */
  list: Accessor<T[] | undefined>
  /** Whether `list`'s query is currently refetching. */
  isFetching: Accessor<boolean>
  /** Whether `item` is the entity `id` refers to. */
  matches: (item: T, id: number) => boolean
  /** Where to send the user once `id` is confirmed absent from `list`. */
  redirectTo: string
}

/** Leaves the current page when the entity its route points at is gone —
 *  deleted from under the user, most often from another surface.
 *
 *  Both guards narrow what counts as evidence of "gone", because the raw
 *  signal — "the id is not in the list I can see" — is also what an
 *  unparseable route id and a list that is not current yet look like, and
 *  acting on either navigates the user off a page that is perfectly valid:
 *
 *  - A route id that fails to parse to an integer is not evidence of
 *    deletion, and it matches nothing in any list — skip rather than bounce.
 *  - `list`'s `data` is the last *successful* result and keeps being served
 *    unchanged while a refetch is in flight, so it can predate the entity
 *    currently being viewed. Waiting for `isFetching` to clear costs
 *    nothing: the effect re-runs when the refetch lands, and a genuinely
 *    deleted entity leaves then. */
export const useEntityGoneGuard = <T>(
  options: UseEntityGoneGuardOptions<T>
) => {
  const navigator = useGDNavigate()

  createEffect(() => {
    const id = options.id()

    if (!Number.isInteger(id)) return

    const items = options.list()
    if (!items) return

    if (options.isFetching()) return

    if (!items.some((item) => options.matches(item, id))) {
      navigator.navigate(options.redirectTo)
    }
  })
}
