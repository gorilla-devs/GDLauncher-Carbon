/** Navigation is untrusted until the requested version's owner is checked. */
export function validatedRequestedVersion(
  requested: string | undefined,
  projectId: string | undefined,
  version: { id: string; project_id: string } | undefined
): string | undefined {
  if (!requested) return undefined
  if (!version || !projectId) return undefined
  if (version.id !== requested || version.project_id !== projectId)
    throw new Error(
      "The selected version belongs to a different project. Select a version from this project's Versions tab."
    )
  return requested
}
