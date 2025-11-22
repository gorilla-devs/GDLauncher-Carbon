import { contextBridge, ipcRenderer } from "electron"
import { domReady } from "./utils"

export interface Log {
  type: "info" | "error"
  message: string
}

function pathJoin(...paths: string[]) {
  let pathSep = "/"
  if (process.platform === "win32") {
    pathSep = "\\"
  }

  return paths.join(pathSep)
}

let oDiv: HTMLDivElement
function useLoading() {
  return {
    async fatalError(error: string | Log[], moduleName: string) {
      const userData = await ipcRenderer.invoke("getUserData")
      const initialRuntimePath = await ipcRenderer.invoke(
        "getInitialRuntimePath"
      )
      const runtimePath = await ipcRenderer.invoke("getRuntimePath")
      const isString = typeof error === "string"

      if (Array.isArray(error)) {
        error = error.map((log) => log.message).join("<br /><br />")
      } else {
        error = error.toString()
      }

      oDiv.classList.add("appFatalCrash")

      const _fontSize = isString ? "1.3rem" : "1rem"
      const dbPath = pathJoin(runtimePath, "gdl_conf.db")

      const errorText = `
      <div style="font-size: 0.8rem; font-weight: 300; background: rgb(var(--darkSlate-900)); max-height: 150px; overflow-y: auto; padding: 12px; text-align: left; border-radius: 6px; overflow-wrap: break-word; font-family: 'Ubuntu Mono', monospace; border: 1px solid rgb(var(--darkSlate-600)); line-height: 1.4;">
        ${error}
      </div>`

      oDiv.innerHTML = `
      <div style="height: 100vh; overflow-y: auto; padding: 16px 20px;">
        <div style="max-width: 900px; margin: 0 auto;">
          <div style="text-align: center; margin-bottom: 24px;">
            <div style="font-size: 1.8rem; font-weight: 800; background: linear-gradient(135deg, rgb(var(--primary-400)), rgb(var(--primary-600))); -webkit-background-clip: text; -webkit-text-fill-color: transparent; margin-bottom: 8px;">
              GDLauncher Couldn't Launch
            </div>
            <div style="font-size: 0.9rem; color: rgb(var(--lightSlate-300)); margin-bottom: 4px;">
              Failed to load: <span style="color: rgb(var(--primary-400)); font-weight: 600;">${moduleName}</span> • v${__APP_VERSION__}
            </div>
          </div>

          ${errorText}

          <div style="background: rgb(var(--darkSlate-700)); border-radius: 12px; padding: 16px; border: 1px solid rgb(var(--primary-500)); margin-top: 20px;">
            <div style="display: flex; align-items: center; gap: 12px; margin-bottom: 12px;">
              <span style="color: rgb(var(--lightSlate-700)); font-weight: 600;">1.</span>
              <span style="font-size: 1rem; font-weight: 600; color: rgb(var(--primary-400));">Check for Updates</span>
            </div>
            <div style="display: flex; align-items: center; justify-content: space-between; gap: 12px;">
              <div id="update-status-text" style="font-size: 0.85rem; font-weight: 500; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; flex: 1; text-align: left; color: rgb(var(--lightSlate-300));">
                -
              </div>
              <div id="update-progress-container" style="display: none; flex: 1; max-width: 120px;">
                <div style="background: rgb(var(--darkSlate-600)); border-radius: 3px; height: 4px; overflow: hidden;">
                  <div id="update-progress-bar" style="background: rgb(var(--primary-500)); height: 100%; width: 0%; transition: width 0.3s ease;"></div>
                </div>
              </div>
              <button id="check-updates-btn" style="padding: 8px 16px; background: rgb(var(--primary-500)); color: white; border: none; border-radius: 6px; font-weight: 600; cursor: pointer; font-size: 0.85rem; transition: all 0.2s; white-space: nowrap; flex-shrink: 0;">
                Check for Updates
              </button>
            </div>
          </div>
          <div id="update-container" style="display: none;"></div>
          <div id="update-status-subtext" style="display: none;"></div>
          <div id="update-progress-text" style="display: none;"></div>
          <button id="update-action-btn" style="display: none;"></button>

          <div style="margin-top: 16px;">
            <div style="background: rgb(var(--darkSlate-700)); border-radius: 12px; padding: 16px; border: 1px solid rgb(var(--darkSlate-600));">
              <div style="font-size: 1rem; font-weight: 600; margin-bottom: 12px; color: rgb(var(--primary-400));">
                Additional Troubleshooting
              </div>
              <div style="display: grid; gap: 10px; font-size: 0.85rem; line-height: 1.5;">
              <div style="display: flex; align-items: center; gap: 8px;">
                <span style="color: rgb(var(--lightSlate-700)); font-weight: 500;">2.</span>
                <span style="color: rgb(var(--lightSlate-50));">Restart GDLauncher</span>
                <button id="restart-btn" style="margin-left: auto; padding: 8px 16px; background: rgb(var(--primary-500)); color: white; border: none; border-radius: 6px; font-weight: 600; cursor: pointer; font-size: 0.8rem; transition: all 0.2s;">
                  Restart
                </button>
              </div>
              <div style="display: flex; align-items: center; gap: 8px;">
                <span style="color: rgb(var(--lightSlate-700)); font-weight: 500;">3.</span>
                <span style="color: rgb(var(--lightSlate-300));">Restart your computer</span>
              </div>
              <div style="display: flex; align-items: center; gap: 8px;">
                <span style="color: rgb(var(--lightSlate-700)); font-weight: 500;">4.</span>
                <span style="color: rgb(var(--lightSlate-300));">Reinstall GDLauncher</span>
              </div>
              <div style="display: flex; align-items: center; gap: 8px;">
                <span style="color: rgb(var(--lightSlate-700)); font-weight: 500;">5.</span>
                <span style="color: rgb(var(--lightSlate-50));">Ask for help on Discord</span>
                <button id="discord-btn" style="margin-left: auto; padding: 8px 16px; background: rgb(var(--brands-discord)); color: white; border: none; border-radius: 6px; font-weight: 600; cursor: pointer; font-size: 0.8rem; transition: all 0.2s;">
                  Discord
                </button>
              </div>
              <div style="display: flex; flex-direction: column; gap: 8px; background: rgb(var(--darkSlate-900)); padding: 12px; border-radius: 6px; border: 1px solid rgb(var(--darkSlate-600));">
                <div style="display: flex; align-items: center; gap: 8px;">
                  <span style="color: rgb(var(--lightSlate-700)); font-weight: 500;">6.</span>
                  <span style="font-size: 0.8rem; color: rgb(var(--lightSlate-300));">Delete database (last resort)</span>
                </div>
                <div style="margin-left: 20px; font-size: 0.75rem; color: rgb(var(--lightSlate-700)); line-height: 1.4;">
                  This will reset all settings and log you out. Your instances and game files will not be affected.
                </div>
                <div style="display: flex; gap: 6px; margin-left: 20px; flex-wrap: wrap; align-items: center;">
                  <code style="flex: 1; min-width: 250px; background: rgb(var(--darkSlate-800)); padding: 8px 10px; border-radius: 6px; font-size: 0.75rem; font-family: 'Ubuntu Mono', monospace; overflow-wrap: break-word; border: 1px solid rgb(var(--darkSlate-600)); color: rgb(var(--lightSlate-300));">
                    ${dbPath}
                  </code>
                  <button id="open-db-folder-btn" style="padding: 6px 12px; background: rgb(var(--darkSlate-600)); color: white; border: 2px solid rgb(var(--darkSlate-500)); border-radius: 6px; font-weight: 600; cursor: pointer; font-size: 0.75rem; transition: all 0.2s; white-space: nowrap;">
                    Open
                  </button>
                  <button id="copy-db-path-btn" style="padding: 6px 12px; background: rgb(var(--darkSlate-600)); color: white; border: 2px solid rgb(var(--darkSlate-500)); border-radius: 6px; font-weight: 600; cursor: pointer; font-size: 0.75rem; transition: all 0.2s; white-space: nowrap;">
                    Copy
                  </button>
                </div>
              </div>
            </div>
          </div>

          <button id="copy-error-btn" style="margin-top: 12px; padding: 8px 16px; background: rgb(var(--darkSlate-700)); color: rgb(var(--lightSlate-50)); border: 1px solid rgb(var(--darkSlate-600)); border-radius: 6px; font-weight: 600; cursor: pointer; font-size: 0.8rem; transition: all 0.2s; width: 100%;">
            Copy Full Error Log
          </button>
        </div>
      </div>
    </div>`

      const restartBtn: HTMLButtonElement =
        document.querySelector("#restart-btn")!
      const discordBtn: HTMLButtonElement =
        document.querySelector("#discord-btn")!
      const openDbFolderBtn: HTMLButtonElement = document.querySelector(
        "#open-db-folder-btn"
      )!
      const copyDbPathBtn: HTMLButtonElement =
        document.querySelector("#copy-db-path-btn")!
      const copyErrorBtn: HTMLButtonElement =
        document.querySelector("#copy-error-btn")!
      const checkUpdatesBtn: HTMLButtonElement =
        document.querySelector("#check-updates-btn")!

      const _updateContainer: HTMLElement =
        document.querySelector("#update-container")!
      const updateActionBtn: HTMLButtonElement =
        document.querySelector("#update-action-btn")!
      const updateStatusText: HTMLElement = document.querySelector(
        "#update-status-text"
      )!
      const _updateStatusSubtext: HTMLElement = document.querySelector(
        "#update-status-subtext"
      )!
      const updateProgressContainer: HTMLElement = document.querySelector(
        "#update-progress-container"
      )!
      const updateProgressBar: HTMLElement = document.querySelector(
        "#update-progress-bar"
      )!
      const _updateProgressText: HTMLElement = document.querySelector(
        "#update-progress-text"
      )!

      let updateInfo: any = null
      let updateState:
        | "idle"
        | "checking"
        | "downloading"
        | "ready"
        | "error"
        | "no-update" = "idle"

      const setUpdateState = (state: typeof updateState, data?: any) => {
        updateState = state

        switch (state) {
          case "idle":
            updateStatusText.textContent = "-"
            updateStatusText.style.whiteSpace = "nowrap"
            updateProgressContainer.style.display = "none"
            checkUpdatesBtn.textContent = "Check for Updates"
            checkUpdatesBtn.disabled = false
            checkUpdatesBtn.style.opacity = "1"
            checkUpdatesBtn.style.background = "rgb(var(--primary-500))"
            break

          case "checking":
            updateStatusText.textContent = "Checking for updates..."
            updateStatusText.style.whiteSpace = "nowrap"
            updateProgressContainer.style.display = "none"
            checkUpdatesBtn.textContent = "Checking..."
            checkUpdatesBtn.disabled = true
            checkUpdatesBtn.style.opacity = "0.6"
            break

          case "downloading":
            if (data) {
              updateInfo = data
            }
            updateStatusText.textContent = `Downloading v${updateInfo?.version || "..."}`
            updateStatusText.style.whiteSpace = "nowrap"
            updateProgressContainer.style.display = "block"
            checkUpdatesBtn.textContent = "Downloading..."
            checkUpdatesBtn.disabled = true
            checkUpdatesBtn.style.opacity = "0.6"
            break

          case "ready":
            updateStatusText.textContent = `Update v${updateInfo?.version || "..."} ready`
            updateStatusText.style.whiteSpace = "nowrap"
            updateProgressContainer.style.display = "none"
            checkUpdatesBtn.textContent = "Apply & Restart"
            checkUpdatesBtn.disabled = false
            checkUpdatesBtn.style.opacity = "1"
            checkUpdatesBtn.style.background = "rgb(var(--green-500))"
            break

          case "no-update": {
            updateStatusText.innerHTML =
              'No update available. Please <a href="#" id="discord-link-inline" style="color: rgb(var(--primary-400)); text-decoration: underline; cursor: pointer;">report this issue on Discord</a> and check again later.'
            updateStatusText.style.whiteSpace = "normal"
            updateProgressContainer.style.display = "none"
            checkUpdatesBtn.textContent = "Check for Updates"
            checkUpdatesBtn.disabled = false
            checkUpdatesBtn.style.opacity = "1"
            checkUpdatesBtn.style.background = "rgb(var(--primary-500))"

            const discordLinkInline: HTMLElement | null =
              document.querySelector("#discord-link-inline")
            if (discordLinkInline) {
              discordLinkInline.onclick = (e: Event) => {
                e.preventDefault()
                ipcRenderer.invoke(
                  "openExternalLink",
                  "https://discord.gdlauncher.com"
                )
              }
            }
            break
          }

          case "error":
            updateStatusText.textContent =
              data?.message || "Update check failed"
            updateStatusText.style.whiteSpace = "nowrap"
            updateProgressContainer.style.display = "none"
            checkUpdatesBtn.textContent = "Retry"
            checkUpdatesBtn.disabled = false
            checkUpdatesBtn.style.opacity = "1"
            checkUpdatesBtn.style.background = "rgb(var(--primary-500))"
            break
        }
      }

      const addHoverEffect = (btn: HTMLButtonElement) => {
        btn.addEventListener("mouseenter", () => {
          if (!btn.disabled) {
            btn.style.filter = "brightness(0.85)"
          }
        })
        btn.addEventListener("mouseleave", () => {
          btn.style.filter = "brightness(1)"
        })
      }

      ;[
        checkUpdatesBtn,
        restartBtn,
        discordBtn,
        openDbFolderBtn,
        copyDbPathBtn,
        copyErrorBtn,
        updateActionBtn
      ].forEach(addHoverEffect)

      const checkForUpdates = async () => {
        setUpdateState("checking")

        try {
          let releaseChannel = "stable"
          try {
            const settings = await ipcRenderer.invoke("getSettings")
            if (settings?.releaseChannel) {
              releaseChannel = settings.releaseChannel
            }
          } catch (_e) {
            // Use stable channel if settings unavailable
          }

          await ipcRenderer.invoke("checkForUpdates", releaseChannel)
        } catch (error) {
          console.error("Update check failed:", error)
          setUpdateState("error", { message: "Failed to check for updates" })
        }
      }

      checkUpdatesBtn.addEventListener("click", async () => {
        if (
          updateState === "idle" ||
          updateState === "error" ||
          updateState === "no-update"
        ) {
          await checkForUpdates()
        } else if (updateState === "ready") {
          await ipcRenderer.invoke("installUpdate")
        }
      })

      restartBtn.addEventListener("click", () => {
        ipcRenderer.invoke("relaunch")
      })

      discordBtn.addEventListener("click", () => {
        ipcRenderer.invoke("openExternalLink", "https://discord.gdlauncher.com")
      })

      openDbFolderBtn.addEventListener("click", async () => {
        await ipcRenderer.invoke("openFolder", dbPath)
        openDbFolderBtn.textContent = "Opened!"
        setTimeout(() => {
          openDbFolderBtn.textContent = "Open"
        }, 2000)
      })

      copyDbPathBtn.addEventListener("click", async () => {
        await navigator.clipboard.writeText(dbPath)
        copyDbPathBtn.textContent = "Copied!"
        setTimeout(() => {
          copyDbPathBtn.textContent = "Copy"
        }, 2000)
      })

      copyErrorBtn.addEventListener("click", async () => {
        const fullLog = `GDLauncher Error Report
Version: ${__APP_VERSION__}
Module: ${moduleName}
User Data: ${userData}
Initial Runtime: ${initialRuntimePath}
Runtime Path: ${runtimePath}
Database Path: ${dbPath}

Error Details:
${error.replace(/<br\s*\/?>/gi, "\n").replace(/<[^>]*>/g, "")}`

        await navigator.clipboard.writeText(fullLog)
        copyErrorBtn.textContent = "Copied!"
        setTimeout(() => {
          copyErrorBtn.textContent = "Copy Full Error Log"
        }, 2000)
      })

      ipcRenderer.on("gd-update-state-changed", (_, stateData: any) => {
        switch (stateData.state) {
          case "idle":
            setUpdateState("idle")
            break

          case "checking":
            setUpdateState("checking")
            break

          case "downloading": {
            const percent = Math.round(stateData.progress || 0)
            updateProgressBar.style.width = `${percent}%`

            if (stateData.updateInfo && updateState !== "downloading") {
              setUpdateState("downloading", stateData.updateInfo)
            }
            break
          }

          case "downloaded":
            setUpdateState("ready")
            break

          case "no-update":
            setUpdateState("no-update")
            break

          case "error":
            setUpdateState("error", stateData.error)
            break
        }
      })

      const currentState = await ipcRenderer.invoke("getUpdateState")
      if (currentState && currentState.state === "idle") {
        checkForUpdates()
      } else if (currentState) {
        switch (currentState.state) {
          case "checking":
            setUpdateState("checking")
            break
          case "downloading":
            if (currentState.updateInfo) {
              updateInfo = currentState.updateInfo
            }
            setUpdateState("downloading", currentState.updateInfo)
            if (currentState.progress) {
              updateProgressBar.style.width = `${Math.round(currentState.progress)}%`
            }
            break
          case "downloaded":
            setUpdateState("ready")
            break
          case "no-update":
            setUpdateState("no-update")
            break
          case "error":
            setUpdateState("error", currentState.error)
            break
        }
      }
    }
  }
}

const { fatalError } = useLoading()

;(async () => {
  await domReady()
  oDiv = document.querySelector("#appFatalCrash")!
})()

contextBridge.exposeInMainWorld("fatalError", fatalError)

contextBridge.exposeInMainWorld(
  "listenToCoreModuleProgress",
  (cb: (event: Electron.IpcRendererEvent, progress: number) => void) =>
    ipcRenderer.on("coreModuleProgress", cb)
)
