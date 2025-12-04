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

      oDiv.innerHTML = `
      <div style="height: 100vh; overflow-y: auto; padding: 16px 20px; text-align: left;">
        <div style="max-width: 600px; margin: 0 auto; display: flex; flex-direction: column; gap: 16px;">

          <!-- Header -->
          <div style="text-align: left;">
            <div style="font-size: 1.6rem; font-weight: 800; background: linear-gradient(135deg, rgb(var(--primary-400)), rgb(var(--primary-600))); -webkit-background-clip: text; -webkit-text-fill-color: transparent; margin-bottom: 6px;">
              GDLauncher Couldn't Launch
            </div>
            <div style="font-size: 0.85rem; color: rgb(var(--lightSlate-400));">
              Failed to load: <span style="color: rgb(var(--primary-400)); font-weight: 600;">${moduleName}</span> • v${__APP_VERSION__}
            </div>
          </div>

          <!-- Error Details -->
          <div style="display: flex; flex-direction: column; gap: 6px; text-align: left;">
            <div style="display: flex; align-items: center; justify-content: space-between;">
              <span style="font-size: 0.8rem; font-weight: 600; color: rgb(var(--lightSlate-400));">Error Details</span>
              <button id="copy-error-detail-btn" style="padding: 4px 10px; background: rgb(var(--darkSlate-600)); color: rgb(var(--lightSlate-50)); border: 1px solid rgb(var(--darkSlate-500)); border-radius: 4px; font-weight: 500; cursor: pointer; font-size: 0.7rem; transition: all 0.2s;">
                Copy
              </button>
            </div>
            <div id="error-detail-content" style="font-size: 0.75rem; font-weight: 300; background: rgb(var(--darkSlate-900)); max-height: 120px; overflow-y: auto; padding: 10px; text-align: left; border-radius: 6px; overflow-wrap: break-word; font-family: 'Ubuntu Mono', monospace; border: 1px solid rgb(var(--darkSlate-600)); line-height: 1.4;">
              ${error}
            </div>
          </div>

          <!-- Step 1: Restart -->
          <div style="background: rgb(var(--darkSlate-700)); border-radius: 10px; padding: 14px; border: 1px solid rgb(var(--darkSlate-600)); text-align: left;">
            <div style="font-size: 0.7rem; font-weight: 700; color: rgb(var(--lightSlate-500)); text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 6px;">Step 1</div>
            <div style="display: flex; align-items: center; justify-content: space-between; gap: 12px;">
              <div style="text-align: left;">
                <div style="font-size: 0.95rem; font-weight: 600; color: rgb(var(--lightSlate-50)); margin-bottom: 2px;">Restart GDLauncher</div>
                <div style="font-size: 0.8rem; color: rgb(var(--lightSlate-400));">A simple restart often fixes temporary issues.</div>
              </div>
              <button id="restart-btn" style="padding: 8px 16px; background: rgb(var(--primary-500)); color: white; border: none; border-radius: 6px; font-weight: 600; cursor: pointer; font-size: 0.8rem; transition: all 0.2s; white-space: nowrap; flex-shrink: 0;">
                Restart
              </button>
            </div>
          </div>

          <!-- Step 2: Check for Updates -->
          <div style="background: rgb(var(--darkSlate-700)); border-radius: 10px; padding: 14px; border: 1px solid rgb(var(--darkSlate-600)); text-align: left;">
            <div style="font-size: 0.7rem; font-weight: 700; color: rgb(var(--lightSlate-500)); text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 6px;">Step 2</div>
            <div style="display: flex; align-items: center; justify-content: space-between; gap: 12px;">
              <div style="flex: 1; min-width: 0; text-align: left;">
                <div style="font-size: 0.95rem; font-weight: 600; color: rgb(var(--lightSlate-50)); margin-bottom: 2px;">Check for Updates</div>
                <div id="update-status-text" style="font-size: 0.8rem; color: rgb(var(--lightSlate-400)); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                  An update may have fixed this issue.
                </div>
                <div id="update-progress-container" style="display: none; margin-top: 6px;">
                  <div style="background: rgb(var(--darkSlate-600)); border-radius: 3px; height: 4px; overflow: hidden;">
                    <div id="update-progress-bar" style="background: rgb(var(--primary-500)); height: 100%; width: 0%; transition: width 0.3s ease;"></div>
                  </div>
                </div>
              </div>
              <button id="check-updates-btn" style="padding: 8px 16px; background: rgb(var(--primary-500)); color: white; border: none; border-radius: 6px; font-weight: 600; cursor: pointer; font-size: 0.8rem; transition: all 0.2s; white-space: nowrap; flex-shrink: 0;">
                Check for Updates
              </button>
            </div>
          </div>
          <div id="update-container" style="display: none;"></div>
          <div id="update-status-subtext" style="display: none;"></div>
          <div id="update-progress-text" style="display: none;"></div>
          <button id="update-action-btn" style="display: none;"></button>

          <!-- Step 3: Reset Database (Highlighted) -->
          <div style="background: rgb(var(--darkSlate-700)); border-radius: 10px; padding: 14px; border: 1px solid rgb(var(--amber-600)); text-align: left;">
            <div style="display: flex; align-items: center; gap: 10px; margin-bottom: 8px;">
              <span style="font-size: 0.7rem; font-weight: 700; color: rgb(var(--lightSlate-500)); text-transform: uppercase; letter-spacing: 0.5px;">Step 3</span>
              <span style="font-size: 0.65rem; font-weight: 600; color: rgb(var(--amber-400)); background: rgba(251, 191, 36, 0.15); padding: 2px 8px; border-radius: 4px; text-transform: uppercase; letter-spacing: 0.3px;">Most common fix</span>
            </div>
            <div style="font-size: 0.95rem; font-weight: 600; color: rgb(var(--lightSlate-50)); margin-bottom: 4px;">Reset Database</div>
            <div style="font-size: 0.8rem; color: rgb(var(--lightSlate-400)); margin-bottom: 12px;">This fixes most launch issues. Your settings will be reset but <strong style="color: rgb(var(--lightSlate-200));">instances are NOT affected</strong>.</div>
            <div style="display: flex; gap: 8px; align-items: center; flex-wrap: wrap;">
              <code style="flex: 1; min-width: 180px; background: rgb(var(--darkSlate-800)); padding: 8px 10px; border-radius: 6px; font-size: 0.7rem; font-family: 'Ubuntu Mono', monospace; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; border: 1px solid rgb(var(--darkSlate-600)); color: rgb(var(--lightSlate-400)); text-align: left;">
                ${dbPath}
              </code>
              <button id="open-db-folder-btn" style="padding: 8px 12px; background: transparent; color: rgb(var(--lightSlate-300)); border: 1px solid rgb(var(--darkSlate-500)); border-radius: 6px; font-weight: 500; cursor: pointer; font-size: 0.75rem; transition: all 0.2s; white-space: nowrap;">
                Open Folder
              </button>
              <button id="reset-db-btn" style="padding: 8px 14px; background: rgb(var(--darkSlate-600)); color: rgb(var(--lightSlate-50)); border: 1px solid rgb(var(--darkSlate-500)); border-radius: 6px; font-weight: 600; cursor: pointer; font-size: 0.75rem; transition: all 0.2s; white-space: nowrap;">
                Reset & Restart
              </button>
            </div>
          </div>

          <!-- Discord Help -->
          <div style="background: rgb(var(--darkSlate-700)); border-radius: 10px; padding: 14px; border: 1px solid rgb(var(--brands-discord)); text-align: left;">
            <div style="display: flex; align-items: center; justify-content: space-between; gap: 12px;">
              <div style="text-align: left;">
                <div style="font-size: 0.95rem; font-weight: 600; color: rgb(var(--lightSlate-50)); margin-bottom: 2px;">Need Help?</div>
                <div style="font-size: 0.8rem; color: rgb(var(--lightSlate-400));">Join our Discord community for support.</div>
              </div>
              <button id="discord-btn" style="padding: 8px 16px; background: rgb(var(--brands-discord)); color: white; border: none; border-radius: 6px; font-weight: 600; cursor: pointer; font-size: 0.8rem; transition: all 0.2s; white-space: nowrap; flex-shrink: 0;">
                Discord
              </button>
            </div>
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
      const resetDbBtn: HTMLButtonElement =
        document.querySelector("#reset-db-btn")!
      const copyErrorDetailBtn: HTMLButtonElement =
        document.querySelector("#copy-error-detail-btn")!
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
            updateStatusText.textContent = "An update may have fixed this issue."
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
        resetDbBtn,
        copyErrorDetailBtn,
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
          openDbFolderBtn.textContent = "Open Folder"
        }, 2000)
      })

      resetDbBtn.addEventListener("click", async () => {
        resetDbBtn.textContent = "Resetting..."
        resetDbBtn.disabled = true
        resetDbBtn.style.opacity = "0.6"
        await ipcRenderer.invoke("deleteDbAndRestart")
      })

      copyErrorDetailBtn.addEventListener("click", async () => {
        const errorDetail = error.replace(/<br\s*\/?>/gi, "\n").replace(/<[^>]*>/g, "")
        await navigator.clipboard.writeText(errorDetail)
        copyErrorDetailBtn.textContent = "Copied!"
        setTimeout(() => {
          copyErrorDetailBtn.textContent = "Copy"
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
            if (stateData.updateInfo) {
              updateInfo = stateData.updateInfo
            }
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
            if (currentState.updateInfo) {
              updateInfo = currentState.updateInfo
            }
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
