import {
  createContext,
  createEffect,
  createSignal,
  For,
  useContext,
  JSX,
  Switch,
  Match
} from "solid-js"
import { createStore, produce } from "solid-js/store"
import { Tooltip, TooltipContent, TooltipTrigger } from "../Tooltip"
import { Portal } from "solid-js/web"

export type NotificationType = "success" | "warning" | "error"
// type NotificationPosition = "bottom" | "top";

export interface Notification {
  id: number
  name: string
  content?: string | JSX.Element
  type?: NotificationType
  // position?: NotificationPosition;
  duration: number
  remainingDuration: number
  progressAnimationId?: number
  progressStart?: number
  expanded: boolean
  timerId?: number
  copied: boolean
  fadingOut: boolean
}

export interface NotificationT {
  name: string
  content?: string | JSX.Element
  type?: NotificationType
  // position?: NotificationPosition,
  duration?: number
}

const NotificationContext = createContext<(_: NotificationT) => void>()

interface Props {
  children: JSX.Element
}

const NotificationsProvider = (props: Props) => {
  const [notifications, setNotifications] = createStore<
    Record<number, Notification>
  >({})
  const [windowFocused, setWindowFocused] = createSignal(true)
  const [isHovering, setIsHovering] = createSignal(false)

  createEffect(() => {
    const onFocus = () => {
      setWindowFocused(true)
      for (const notification of Object.values(notifications)) {
        if (isHovering()) continue

        if (notification.remainingDuration > 0) {
          resumeTimer(notification)
        }
      }
    }
    const onBlur = () => {
      setWindowFocused(false)
      for (const notification of Object.values(notifications)) {
        if (notification.remainingDuration > 0) {
          pauseTimer(notification)
        }
      }
    }

    window.addEventListener("focus", onFocus)
    window.addEventListener("blur", onBlur)

    return () => {
      window.removeEventListener("focus", onFocus)
      window.removeEventListener("blur", onBlur)
    }
  })

  const removeNotification = (id: number) => {
    const notification = notifications[id]

    if (!notification) return

    if (notification.timerId) {
      clearTimeout(notification.timerId)
    }
    if (notification.progressAnimationId) {
      cancelAnimationFrame(notification.progressAnimationId)
    }

    setNotifications(id, "fadingOut", true)

    setTimeout(() => {
      setNotifications(produce((prev) => delete prev[id]))
    }, 170)
  }

  function animateBar(ts: number, id: number) {
    const notification = notifications[id]
    const now = Date.now()
    const delta = now - ts

    if (!notification || delta < 5) {
      requestAnimationFrame(() => animateBar(ts, id))
      return
    }

    const newRemainingTime = Math.ceil(
      Math.max(0, notification.remainingDuration - delta)
    )

    setNotifications(notification.id, "remainingDuration", newRemainingTime)

    if (newRemainingTime > 0) {
      setNotifications(
        notification.id,
        "progressAnimationId",
        requestAnimationFrame(() => animateBar(now, notification.id))
      )
    }
  }

  const addNotification = ({
    name,
    content,
    type,
    duration = 7000
  }: NotificationT) => {
    const id = Date.now()

    const newNotification: Notification = {
      id,
      name,
      content,
      type,
      duration,
      remainingDuration: duration,
      expanded: false,
      copied: false,
      fadingOut: false
    }

    newNotification.progressStart = Date.now()
    newNotification.progressAnimationId = requestAnimationFrame(() => {
      animateBar(newNotification.progressStart!, newNotification.id)
    })

    newNotification.timerId = window.setTimeout(() => {
      removeNotification(id)
    }, duration)

    setNotifications(id, newNotification)
  }

  const pauseTimer = (notification: Notification) => {
    if (notification.timerId) {
      clearTimeout(notification.timerId)
      setNotifications(notification.id, "timerId", undefined)
    }
    if (notification.progressAnimationId) {
      cancelAnimationFrame(notification.progressAnimationId)
      setNotifications(notification.id, "progressAnimationId", undefined)
      setNotifications(notification.id, "progressStart", undefined)
    }
  }

  const resumeTimer = (notification: Notification) => {
    const newTimerId = window.setTimeout(() => {
      removeNotification(notification.id)
    }, notification.remainingDuration)

    const newProgressStart = Date.now()
    const newProgressId = requestAnimationFrame(() => {
      animateBar(newProgressStart, notification.id)
    })

    setNotifications(notification.id, "timerId", newTimerId)
    setNotifications(notification.id, "progressStart", newProgressStart)
    setNotifications(notification.id, "progressAnimationId", newProgressId)
  }

  return (
    <NotificationContext.Provider value={addNotification}>
      <Portal mount={document.getElementById("notifications")!}>
        <div
          class="flex flex-col gap-4 overflow-y-auto p-5"
          style={{
            "max-height": "calc(100vh - 1.25rem)"
          }}
          onMouseEnter={() => {
            setIsHovering(true)

            const areAllValid = Object.values(notifications).every(
              (notification) => !notification.copied
            )

            if (areAllValid && windowFocused()) {
              for (const notification of Object.values(notifications)) {
                if (notification.remainingDuration > 0) {
                  pauseTimer(notification)
                }
              }
            }
          }}
          onMouseLeave={() => {
            setIsHovering(false)

            const areAllValid = Object.values(notifications).every(
              (notification) => !notification.copied
            )

            if (areAllValid && windowFocused()) {
              for (const notification of Object.values(notifications)) {
                if (notification.remainingDuration > 0) {
                  resumeTimer(notification)
                }
              }
            }
          }}
        >
          <For each={Object.values(notifications)}>
            {(notification) => (
              <div
                class="w-100 h-26 min-h-26 bg-darkSlate-900 text-lightSlate-50 shadow-darkSlate-900 relative flex items-center justify-between rounded-lg px-4 py-4 shadow-md"
                style={{
                  transition: "all 0.3s ease-in-out",
                  animation: `.2s ease-in-out ${
                    notification.fadingOut ? "fadeOut" : "fadeIn"
                  }`
                }}
                classList={{
                  "h-68 min-h-68": notification.expanded
                }}
              >
                <div
                  class="absolute left-0 top-0 h-1 w-full"
                  classList={{
                    "bg-red-400": notification.type === "error",
                    "bg-yellow-400": notification.type === "warning",
                    "bg-green-400": notification.type === "success"
                  }}
                  style={{
                    "border-radius": "0.5rem 0.5rem 0 0",
                    width: `${
                      (notification.remainingDuration / notification.duration) *
                      100
                    }%`
                  }}
                />

                <div
                  class="text-darkSlate-300 hover:text-lightSlate-100 absolute left-4 top-4 h-6 w-6 duration-100 ease-in-out"
                  classList={{
                    "i-ri:arrow-up-s-line": !notification.expanded,
                    "i-ri:arrow-down-s-line": notification.expanded
                  }}
                  onClick={() => {
                    setNotifications(
                      notification.id,
                      "expanded",
                      (prev) => !prev
                    )
                  }}
                />
                <Switch>
                  <Match
                    when={
                      notification.type === "error" ||
                      notification.type === "warning"
                    }
                  >
                    <div class="i-ri:error-warning-fill h-6 w-6 min-w-6 text-red-400" />
                  </Match>
                  <Match when={notification.type === "success"}>
                    <div class="i-ri:check-fill h-6 w-6 min-w-6 text-green-400" />
                  </Match>
                </Switch>
                <div class="m-4 h-full w-auto overflow-y-auto text-left">
                  <div class="pb-2 text-xl font-bold">{notification.name}</div>
                  <div class="text-md">
                    {notification.content || notification.name}
                  </div>
                </div>
                <div class="flex h-full flex-row gap-4">
                  <div class="bg-darkSlate-700 h-full w-[1px] min-w-[1px]" />
                  <div class="flex flex-col justify-between">
                    <div
                      class="text-darkSlate-300 i-ri:close-fill hover:text-lightSlate-100 h-6 w-6 duration-100 ease-in-out"
                      onClick={() => {
                        removeNotification(notification.id)
                      }}
                    />
                    <Tooltip>
                      <TooltipTrigger>
                        <div
                          class="h-6 w-6"
                          classList={{
                            "text-darkSlate-300 hover:text-lightSlate-100 duration-100 ease-in-out i-ri:file-copy-2-fill":
                              !notification.copied,
                            "text-green-400 i-ri:checkbox-circle-fill":
                              notification.copied
                          }}
                          onClick={() => {
                            navigator.clipboard
                              .writeText(notification.name || "")
                              .catch(console.error)

                            setNotifications(notification.id, "copied", true)

                            for (const notification of Object.values(
                              notifications
                            )) {
                              if (notification.remainingDuration > 0) {
                                pauseTimer(notification)
                              }
                            }

                            setTimeout(() => {
                              setNotifications(notification.id, "copied", false)

                              const areAllValid = Object.values(
                                notifications
                              ).every((notification) => !notification.copied)

                              if (
                                areAllValid &&
                                !isHovering() &&
                                windowFocused()
                              ) {
                                for (const notification of Object.values(
                                  notifications
                                )) {
                                  if (notification.remainingDuration > 0) {
                                    resumeTimer(notification)
                                  }
                                }
                              }
                            }, 2000)
                          }}
                        />
                      </TooltipTrigger>
                      <TooltipContent>
                        <p>Copy</p>
                      </TooltipContent>
                    </Tooltip>
                  </div>
                </div>
              </div>
            )}
          </For>
        </div>
      </Portal>
      {props.children}
    </NotificationContext.Provider>
  )
}

const createNotification = () => {
  const context = useContext(NotificationContext)
  if (context === undefined) {
    throw new Error(
      "`createNotification` must be used within a `NotificationsProvider`."
    )
  }
  return context
}

export { NotificationsProvider, createNotification }
