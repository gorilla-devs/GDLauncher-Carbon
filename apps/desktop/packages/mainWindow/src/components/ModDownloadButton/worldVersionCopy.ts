/** Keep action/disclosure and completion language tied to the actual request.
 * The caller snapshots this before mutation because cache rows may refresh.
 */
export function worldVersionCopy(
  addonType: string | undefined,
  fileId: number | string | undefined,
  replacesMod: string | null | undefined,
  serverId?: number
) {
  return addonType === "world" && fileId != null && !!replacesMod && !serverId
    ? {
        action: "content:_trn_world_version_copy_action" as const,
        disclosure: "content:_trn_world_version_copy_disclosure" as const,
        completion: "content:_trn_world_version_copy_complete" as const
      }
    : undefined
}
