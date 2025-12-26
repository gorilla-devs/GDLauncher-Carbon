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
      const copyErrorDetailBtn: HTMLButtonElement = document.querySelector(
        "#copy-error-detail-btn"
      )!
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
            updateStatusText.textContent =
              "An update may have fixed this issue."
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
        const errorDetail = error
          .replace(/<br\s*\/?>/gi, "\n")
          .replace(/<[^>]*>/g, "")
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
    },

    async backwardsMigrationError() {
      oDiv.classList.add("appFatalCrash")

      oDiv.innerHTML = `
      <div style="height: 100vh; overflow-y: auto; padding: 32px 20px; display: flex; flex-direction: column; align-items: center;">
        <!-- Logo -->
        <div style="flex: 1; display: flex; align-items: center; max-height: 35%;">
          <div>
            <svg width="350" height="84" viewBox="0 0 266 64" fill="none" xmlns="http://www.w3.org/2000/svg">
              <path d="M56 20.6428C56 17.7718 54.4616 15.121 51.9688 13.6967L31.9664 2.2679C29.5069 0.862595 26.4876 0.862721 24.0282 2.26823L4.03059 13.6966C1.53816 15.121 0 17.7716 0 20.6424V43.3576C0 46.2284 1.53816 48.879 4.03059 50.3034L24.0282 61.7318C26.4876 63.1373 29.5069 63.1374 31.9664 61.7321L51.9688 50.3033C54.4616 48.879 56 46.2282 56 43.3572V20.6428Z" fill="#2B6CB0"/>
              <path d="M27.9963 47.649C32.7668 47.649 36.6351 43.8202 36.6351 39.0985V37.647C36.6351 37.521 36.7126 37.4114 36.8288 37.3676C38.1681 36.8363 46.0764 33.3471 45.036 25.6292C43.9125 17.3142 33.4752 18.6179 28.2122 23.586C28.0904 23.7011 27.9078 23.7011 27.7916 23.586C22.5286 18.6179 12.0912 17.3142 10.9678 25.6292C9.92184 33.3471 17.8301 36.8363 19.1749 37.3676C19.2911 37.4114 19.3686 37.5265 19.3686 37.647V39.0985C19.3631 43.8257 23.2259 47.649 27.9963 47.649Z" fill="white"/>
              <path d="M24.304 31.1724C24.3317 31.4682 23.9609 31.6271 23.7672 31.4025C23.1363 30.674 21.9299 29.6387 19.9044 29.08C16.5839 28.1707 20.1036 26.2317 21.465 26.8616C22.6936 27.4367 24.0494 28.2365 24.304 31.1724Z" fill="#2B6CB0"/>
              <path d="M31.6977 31.1726C31.6701 31.4684 32.0409 31.6273 32.2346 31.4027C32.8654 30.6742 34.0719 29.6389 36.0974 29.0802C39.4179 28.1709 35.8982 26.2318 34.5368 26.8618C33.3082 27.4369 31.9523 28.2366 31.6977 31.1726Z" fill="#2B6CB0"/>
              <path d="M29.9536 35.5269C30.4074 36.5183 31.1323 38.5012 30.5014 39.9034C30.4129 40.1006 30.1417 40.1335 29.9978 39.9746C29.4776 39.394 28.5534 37.9425 29.3836 35.5597C29.4776 35.2968 29.8429 35.2804 29.9536 35.5269Z" fill="#2B6CB0"/>
              <path d="M26.0455 35.5269C25.5917 36.5183 24.8668 38.5012 25.4977 39.9035C25.5862 40.1007 25.8574 40.1335 26.0013 39.9747C26.5215 39.394 27.4457 37.9425 26.6156 35.5597C26.527 35.2968 26.1618 35.2804 26.0455 35.5269Z" fill="#2B6CB0"/>
              <path d="M76.4 44.3875C81.4995 44.3875 86.878 40.7605 86.878 33.522C86.878 32.902 86.847 32.251 86.754 31.569H75.966V35.878H81.05C80.43 38.0015 78.8645 39.071 76.1365 39.071C73.362 39.071 70.8045 37.1025 70.8045 33.15C70.8045 29.7555 72.9745 27.3995 76.0435 27.3995C77.888 27.3995 79.2055 28.035 80.182 28.996L84.336 24.904C82.4915 23.0285 79.841 21.897 76.4 21.897C69.58 21.897 64.775 26.547 64.775 33.15C64.775 39.7375 69.58 44.3875 76.4 44.3875ZM98.7726 44C105.562 44 110.026 39.691 110.026 33.15C110.026 26.609 105.562 22.3 98.7726 22.3H89.1936V44H98.7726ZM95.0836 27.7095H98.6176C102.183 27.7095 103.903 29.9725 103.903 33.15C103.903 36.281 102.183 38.5905 98.6176 38.5905H95.0836V27.7095ZM128.69 44V38.575H118.243V22.3H112.353V44H128.69ZM136.79 44.403C138.727 44.403 140.06 43.6745 140.975 42.6515V44H146.276V28.6395H140.975V30.019C140.06 28.965 138.727 28.221 136.805 28.221C132.682 28.221 129.799 31.8635 129.799 36.2965C129.799 40.7605 132.682 44.403 136.79 44.403ZM138.216 39.2725C136.635 39.2725 135.488 38.0325 135.488 36.312C135.488 34.5915 136.635 33.3515 138.216 33.3515C139.797 33.3515 140.944 34.5915 140.944 36.312C140.944 38.0325 139.797 39.2725 138.216 39.2725ZM156.419 44.403C160.713 44.403 163.952 42.171 163.952 37.397V28.6395H158.434V36.9165C158.434 38.451 157.628 39.2725 156.419 39.2725C155.21 39.2725 154.404 38.451 154.404 36.9165V28.6395H148.886V37.3815C148.886 42.171 152.126 44.403 156.419 44.403ZM172.101 44V35.7695C172.101 34.111 172.922 33.3825 174.023 33.3825C175.232 33.3825 175.96 34.204 175.96 35.7385V44H181.478V34.0025C181.478 30.4685 179.169 28.221 175.991 28.221C174.069 28.221 172.705 28.8875 171.837 30.2205V28.6395H166.583V44H172.101ZM191.784 44.403C194.853 44.403 197.193 42.977 198.697 40.714L194.155 37.8155C193.489 38.8075 192.652 39.2725 191.613 39.2725C190.265 39.2725 189.056 38.2495 189.056 36.312C189.056 34.39 190.311 33.3515 191.66 33.3515C192.745 33.3515 193.504 33.8165 194.155 34.793L198.681 31.941C197.162 29.647 194.853 28.221 191.784 28.221C186.87 28.221 183.414 31.569 183.414 36.312C183.414 41.055 186.87 44.403 191.784 44.403ZM205.795 44V35.7695C205.795 34.08 206.648 33.3825 207.717 33.3825C208.926 33.3825 209.655 34.204 209.655 35.7385V44H215.173V34.0025C215.173 30.4685 212.863 28.221 209.686 28.221C207.95 28.221 206.663 28.7635 205.795 29.8485V21.649H200.277V44H205.795ZM225.308 44.403C228.346 44.403 230.764 43.349 232.252 41.737L228.47 38.575C227.633 39.3965 226.563 39.815 225.571 39.815C224.114 39.815 223.184 39.195 222.812 37.955H232.841C232.903 37.645 232.949 36.839 232.949 36.312C232.934 31.507 229.741 28.221 225.091 28.221C220.363 28.221 217.108 31.5845 217.108 36.4515C217.108 41.21 220.394 44.403 225.308 44.403ZM222.828 34.6225C223.184 33.429 223.944 32.8555 225.122 32.8555C226.269 32.8555 227.044 33.429 227.447 34.6225H222.828ZM240.474 44V35.7695C240.474 34.08 241.404 33.5375 242.938 33.5375C243.822 33.5375 244.721 33.7235 245.356 33.894L245.666 28.376C245.449 28.3295 244.891 28.221 244.287 28.221C242.411 28.221 241.063 28.8255 240.21 30.1895V28.6395H234.94V44H240.474Z" fill="#2B6CB0"/>
            </svg>
          </div>
        </div>

        <!-- Content -->
        <div style="flex: 1; display: flex; flex-direction: column; align-items: center; max-width: 500px; text-align: center;">
          <!-- Title -->
          <div style="font-size: 2rem; font-weight: 800; color: rgb(var(--lightSlate-50)); margin-bottom: 40px;">
            Database Version Mismatch
          </div>

          <!-- Main message -->
          <div style="font-size: 1rem; color: rgb(var(--lightSlate-300)); line-height: 1.7; margin-bottom: 16px;">
            Your database was created by a newer version of GDLauncher.
          </div>

          <!-- Reassurance -->
          <div style="font-size: 1rem; color: rgb(var(--green-400)); font-weight: 500; margin-bottom: 40px;">
            Your instances and mods will NOT be affected.
          </div>

          <!-- Button -->
          <button id="reset-db-btn" style="padding: 14px 32px; background: rgb(var(--primary-500)); color: white; border: none; border-radius: 8px; font-weight: 600; cursor: pointer; font-size: 1rem; transition: all 0.2s;">
            Reset Database & Restart
          </button>

          <!-- Note -->
          <div style="font-size: 0.8rem; color: rgb(var(--lightSlate-500)); margin-top: 16px;">
            You will need to re-login and reconfigure settings like theme and Java paths.
          </div>

          <!-- Version -->
          <div style="font-size: 0.7rem; color: rgb(var(--lightSlate-600)); margin-top: auto; padding-top: 32px;">
            v${__APP_VERSION__}
          </div>
        </div>
      </div>`

      const resetDbBtn: HTMLButtonElement =
        document.querySelector("#reset-db-btn")!

      resetDbBtn.addEventListener("mouseenter", () => {
        resetDbBtn.style.filter = "brightness(0.85)"
      })
      resetDbBtn.addEventListener("mouseleave", () => {
        resetDbBtn.style.filter = "brightness(1)"
      })

      resetDbBtn.addEventListener("click", async () => {
        resetDbBtn.textContent = "Resetting..."
        resetDbBtn.disabled = true
        resetDbBtn.style.opacity = "0.6"
        await ipcRenderer.invoke("deleteDbAndRestart")
      })
    }
  }
}

const { fatalError, backwardsMigrationError } = useLoading()

;(async () => {
  await domReady()
  oDiv = document.querySelector("#appFatalCrash")!
})()

contextBridge.exposeInMainWorld("fatalError", fatalError)
contextBridge.exposeInMainWorld(
  "backwardsMigrationError",
  backwardsMigrationError
)

contextBridge.exposeInMainWorld(
  "listenToCoreModuleProgress",
  (cb: (event: Electron.IpcRendererEvent, progress: number) => void) =>
    ipcRenderer.on("coreModuleProgress", cb)
)
