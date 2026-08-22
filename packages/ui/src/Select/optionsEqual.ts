/** Structural id-equality for a `Select`'s `options` array.
 *
 *  Kobalte's `Select` keys its rendered listbox items off `options`'
 *  identity, so a caller that recomputes a fresh-but-equivalent array on
 *  every render — a memo invalidated by an unrelated field, a `.map()` over
 *  query data that refetches byte-for-byte identical rows — tears the open
 *  dropdown down and rebuilds it under the user's cursor. Comparing
 *  structurally instead of by reference stops that.
 *
 *  `optionKey` derives the id to compare each option by. Without one,
 *  primitive options (strings, numbers, booleans — every option array in
 *  this codebase today) compare by `===`, and object options fall back to a
 *  JSON-stable serialization of the whole option. */
export function optionsEqual<T>(
  prev: readonly T[],
  next: readonly T[],
  optionKey?: (option: T) => string
): boolean {
  if (prev === next) return true
  if (prev.length !== next.length) return false

  for (let i = 0; i < prev.length; i++) {
    if (!sameOption(prev[i], next[i], optionKey)) return false
  }

  return true
}

function sameOption<T>(a: T, b: T, optionKey?: (option: T) => string): boolean {
  if (a === b) return true

  if (optionKey) return optionKey(a) === optionKey(b)

  if (
    typeof a !== "object" ||
    a === null ||
    typeof b !== "object" ||
    b === null
  ) {
    // Primitives already failed the `===` check above — they differ.
    return false
  }

  try {
    return JSON.stringify(a) === JSON.stringify(b)
  } catch {
    // Circular reference or other non-serializable shape: treat as changed
    // rather than silently coalescing two different options.
    return false
  }
}
