/** Resolves a boolean preference backed by a query that may still be
 *  loading: returns the already-settled value immediately (no unnecessary
 *  await) when the query has resolved, otherwise awaits `ensureResolved`
 *  (expected to prime and return the settled value) instead of treating
 *  "still loading" as "false" — the caller decides what `ensureResolved`
 *  does (e.g. `queryClient.ensureQueryData`). Kept free of app imports so it
 *  can be unit tested without any Solid/router/rspc context. */
export async function resolveBooleanPreference(
  query: { isLoading: boolean; data: boolean | undefined },
  ensureResolved: () => Promise<boolean>
): Promise<boolean> {
  if (!query.isLoading) {
    return query.data === true
  }
  return (await ensureResolved()) === true
}
