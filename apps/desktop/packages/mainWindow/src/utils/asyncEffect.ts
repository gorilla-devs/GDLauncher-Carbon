import { createEffect, onCleanup } from "solid-js"

/**
 * Creates an effect that safely performs async operations with automatic staleness checking.
 * Uses Symbol-based tracking to prevent race conditions where older async operations
 * complete after newer ones, potentially overwriting state with stale data.
 *
 * This abstraction eliminates the need for manual value comparisons and ensures that
 * async callbacks only execute if they belong to the most recent effect run.
 *
 * @param fn - Effect function that receives:
 *   - `isStale`: Function that returns true if a new effect run has started since this one began
 *   - `prev`: The value returned from the previous effect run
 * @param initialValue - Optional initial value for the first effect run's `prev` parameter
 *
 * @returns void - The effect is automatically registered with SolidJS
 *
 * @example
 * // Basic usage - check staleness before updating state
 * createAsyncEffect((isStale) => {
 *   const userId = currentUserId()
 *
 *   fetchUser(userId).then((user) => {
 *     if (!isStale()) {
 *       setUser(user)
 *     }
 *   })
 * })
 *
 * @example
 * // With previous value tracking - only fetch on changes
 * createAsyncEffect((isStale, prevId) => {
 *   const currentId = selectedInstanceId()
 *
 *   if (currentId && currentId !== prevId) {
 *     fetchInstance(currentId).then((instance) => {
 *       if (!isStale()) {
 *         setInstance(instance)
 *       }
 *     })
 *   }
 *
 *   return currentId
 * }, undefined)
 *
 * @example
 * // Multiple async operations
 * createAsyncEffect((isStale) => {
 *   const query = searchQuery()
 *
 *   Promise.all([
 *     fetchResults(query),
 *     fetchFilters(query)
 *   ]).then(([results, filters]) => {
 *     if (!isStale()) {
 *       setResults(results)
 *       setFilters(filters)
 *     }
 *   })
 * })
 *
 * @example
 * // With error handling
 * createAsyncEffect((isStale) => {
 *   const path = currentPath()
 *
 *   scanDirectory(path)
 *     .then((files) => {
 *       if (!isStale()) {
 *         setFiles(files)
 *       }
 *     })
 *     .catch((error) => {
 *       // Error handling can also check staleness if needed
 *       if (!isStale()) {
 *         setError(error)
 *       }
 *     })
 * })
 */
export function createAsyncEffect<T = undefined>(
  fn: (isStale: () => boolean, prev: T | undefined) => T | undefined,
  initialValue?: T
): void {
  let latestSymbol: symbol = Symbol()

  createEffect<T | undefined>((prev) => {
    // Create a unique symbol for this effect run
    const operationSymbol = Symbol()
    latestSymbol = operationSymbol

    // Provide a function to check if this operation is stale
    // Returns true if a new effect run has started since this one began
    const isStale = () => operationSymbol !== latestSymbol

    // Mark as stale when the effect is cleaned up
    // This ensures pending async operations don't update state after cleanup
    onCleanup(() => {
      if (operationSymbol === latestSymbol) {
        latestSymbol = Symbol()
      }
    })

    return fn(isStale, prev)
  }, initialValue)
}
