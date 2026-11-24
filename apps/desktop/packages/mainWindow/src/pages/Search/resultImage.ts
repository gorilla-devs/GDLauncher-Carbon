/** The image a search result shows: the small one for the list's icon, the
 *  large one for a grid tile, each falling back to the other. Null when there
 *  is none, so the row draws a placeholder instead of a broken `src=""`. */
export function resultImageUrl(
  result: { imageUrl: string | null; highResImageUrl: string | null },
  size: "small" | "large"
): string | null {
  const [first, second] =
    size === "small"
      ? [result.imageUrl, result.highResImageUrl]
      : [result.highResImageUrl, result.imageUrl]
  return first || second || null
}
