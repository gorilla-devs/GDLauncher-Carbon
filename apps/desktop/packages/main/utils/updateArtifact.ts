export interface Installation {
  platform: NodeJS.Platform
  arch: string
  format: "zip" | "appimage" | "deb" | "rpm" | "exe" | "dmg" | null
}
export interface ResolvedArtifact {
  url: URL
  info: { url: string }
}
/** Choose actual matching metadata. Missing/ambiguous identity or artifact has no link. */
export function selectUpdateArtifact(
  files: ResolvedArtifact[],
  installation: Installation
): string | null {
  if (!installation.format) return null
  const platform = { win32: "win", darwin: "mac", linux: "linux" }[
    installation.platform as "win32" | "darwin" | "linux"
  ]
  const matches = files.filter((file) => {
    const name = decodeURIComponent(file.url.pathname).toLowerCase()
    if (!name.endsWith(`.${installation.format}`)) return false
    // Builder's actual artifacts encode __OS__ARCH. A metadata channel can
    // contain other architectures; never pick its arbitrary first entry.
    const identity = /__(win|mac|linux)__(x64|arm64|universal)\./.exec(name)
    if (!identity) return false
    return (
      identity[1] === platform &&
      (identity[2] === installation.arch ||
        (installation.platform === "darwin" && identity[2] === "universal"))
    )
  })
  const urls = [...new Set(matches.map((file) => file.url.href))]
  return urls.length === 1 ? urls[0] : null
}
