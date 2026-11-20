/**
 * Whether the filter sidebar has anything to act on.
 *
 * Its filters, list/grid switch and "Clear All" act on search results. A
 * share preview is not one, and a direct lookup (`#id`, a pasted project
 * link) goes by id and takes no filters. The sidebar still stays for a direct
 * lookup made while adding to an instance or server: its header is what names
 * that instance and leads back to it.
 */
export function searchSidebarApplies(search: {
  share: boolean
  direct: boolean
  addingToInstanceOrServer: boolean
}): boolean {
  if (search.share) return false
  if (search.direct) return search.addingToInstanceOrServer
  return true
}
