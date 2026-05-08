import { ipcMain, shell, clipboard } from "electron"

const ALLOWED_OPEN_EXTERNAL_PROTOCOLS = new Set(["http:", "https:", "mailto:"])

function isSafeExternalLink(link: unknown): link is string {
  if (typeof link !== "string") return false
  let url: URL
  try {
    url = new URL(link)
  } catch {
    return false
  }
  return ALLOWED_OPEN_EXTERNAL_PROTOCOLS.has(url.protocol)
}

ipcMain.handle("openExternalLink", async (_, link) => {
  if (!isSafeExternalLink(link)) {
    console.warn("[openExternalLink] rejected unsafe link:", link)
    return
  }
  shell.openExternal(link)
})

ipcMain.handle("copyToClipboard", async (_, text) => {
  clipboard.writeText(text, "clipboard")
})
