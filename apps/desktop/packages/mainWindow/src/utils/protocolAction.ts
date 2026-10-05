import { parseSearchQuery } from "./searchQueryParser"
export function acceptProtocolUrl(
  url: string,
  io: { share(code: string): void; navigate(route: string): void }
) {
  const parsed = parseSearchQuery(url)
  if (parsed.mode !== "direct" || parsed.items.length === 0) return
  const firstItem = parsed.items[0]
  if (firstItem.type === "gdlauncher_share") io.share(firstItem.shareCode)
  else io.navigate(`/search?q=${encodeURIComponent(url)}`)
}
