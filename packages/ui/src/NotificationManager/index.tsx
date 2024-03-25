import {
  createContext,
  createEffect,
  createSignal,
  For,
  JSX,
  useContext,
} from "solid-js";
import { createStore, produce } from "solid-js/store";
import { Tooltip } from "../Tooltip";

type NotificationType = "success" | "warning" | "error";
type NotificationPosition = "bottom" | "top";

type Notification = {
  id: number;
  name: string;
  type?: NotificationType;
  position?: NotificationPosition;
  duration: number;
  remainingDuration: number;
  progressAnimationId?: number;
  progressStart?: number;
  expanded: boolean;
  timerId?: number;
  copied: boolean;
  isMouseInside: boolean;
  fadingOut: boolean;
};

type Props = {
  children: JSX.Element;
};

const NotificationContext =
  createContext<
    (
      _name: string,
      _type?: NotificationType,
      _position?: NotificationPosition,
      _duration?: number
    ) => void
  >();

const NotificationsProvider = (props: Props) => {
  const [notifications, setNotifications] = createStore<
    Record<number, Notification>
  >({});
  const [windowFocused, setWindowFocused] = createSignal(true);

  createEffect(() => {
    const onFocus = () => {
      setWindowFocused(true);
      for (const notification of Object.values(notifications)) {
        if (notification.isMouseInside) continue;

        if (notification.remainingDuration > 0) {
          resumeTimer(notification);
        }
      }
    };
    const onBlur = () => {
      setWindowFocused(false);
      for (const notification of Object.values(notifications)) {
        if (notification.remainingDuration > 0) {
          pauseTimer(notification);
        }
      }
    };

    window.addEventListener("focus", onFocus);
    window.addEventListener("blur", onBlur);

    return () => {
      window.removeEventListener("focus", onFocus);
      window.removeEventListener("blur", onBlur);
    };
  });

  const removeNotification = (id: number) => {
    const notification = notifications[id];

    if (!notification) return;

    if (notification.timerId) {
      clearTimeout(notification.timerId);
    }
    if (notification.progressAnimationId) {
      cancelAnimationFrame(notification.progressAnimationId);
    }

    setNotifications(id, "fadingOut", true);

    setTimeout(() => {
      setNotifications(produce((prev) => delete prev[id]));
    }, 170);
  };

  function animateBar(ts: number, id: number) {
    const notification = notifications[id];
    const now = Date.now();
    const delta = now - ts;

    if (!notification || delta < 5) {
      requestAnimationFrame(() => animateBar(ts, id));
      return;
    }

    const newRemainingTime = Math.ceil(
      Math.max(0, notification.remainingDuration - delta)
    );

    setNotifications(notification.id, "remainingDuration", newRemainingTime);

    if (newRemainingTime > 0) {
      setNotifications(
        notification.id,
        "progressAnimationId",
        requestAnimationFrame(() => animateBar(now, notification.id))
      );
    }
  }

  const addNotification = (
    name: string,
    type: NotificationType = "success",
    position: NotificationPosition = "bottom",
    duration = 7000
  ) => {
    const id = Date.now();

    const newNotification: Notification = {
      id,
      name,
      type,
      position,
      duration,
      remainingDuration: duration,
      expanded: false,
      copied: false,
      isMouseInside: false,
      fadingOut: false,
    };

    newNotification.progressStart = Date.now();
    newNotification.progressAnimationId = requestAnimationFrame(() => {
      animateBar(newNotification.progressStart!, newNotification.id);
    });

    newNotification.timerId = window.setTimeout(() => {
      removeNotification(id);
    }, duration);

    setNotifications(id, newNotification);
  };

  const pauseTimer = (notification: Notification) => {
    if (notification.timerId) {
      clearTimeout(notification.timerId);
      setNotifications(notification.id, "timerId", undefined);
    }
    if (notification.progressAnimationId) {
      cancelAnimationFrame(notification.progressAnimationId);
      setNotifications(notification.id, "progressAnimationId", undefined);
      setNotifications(notification.id, "progressStart", undefined);
    }
  };

  const resumeTimer = (notification: Notification) => {
    const newTimerId = window.setTimeout(() => {
      removeNotification(notification.id);
    }, notification.remainingDuration);

    const newProgressStart = Date.now();
    const newProgressId = requestAnimationFrame(() => {
      animateBar(newProgressStart, notification.id);
    });

    setNotifications(notification.id, "timerId", newTimerId);
    setNotifications(notification.id, "progressStart", newProgressStart);
    setNotifications(notification.id, "progressAnimationId", newProgressId);
  };

  return (
    <NotificationContext.Provider value={addNotification}>
      <For each={Object.values(notifications)}>
        {(notification, i) => (
          <div
            class="min-w-50 w-fit max-w-100 h-26 left-5 items-center bg-darkSlate-900 flex justify-between px-4 py-4 text-white fixed rounded-lg z-100 shadow-md shadow-darkSlate-900"
            style={{
              transform: `translate(0, ${
                notification.position === "bottom"
                  ? `-${i() * 45}`
                  : `${i() * 45}`
              }px)`,
              "overflow-wrap": "anywhere",
              transition: "height 0.3s ease-in-out",
              animation: `.2s ease-in-out ${
                notification.fadingOut ? "fadeOut" : "fadeIn"
              }`,
            }}
            classList={{
              "h-68": notification.expanded,
              "bottom-5": notification.position === "bottom",
              "bottom-auto": notification.position !== "bottom",
              "top-5": notification.position === "top",
              "top-auto": notification.position !== "top",
            }}
            onMouseEnter={() => {
              setNotifications(notification.id, "isMouseInside", true);

              if (
                notification.remainingDuration > 0 &&
                !notification.copied &&
                windowFocused()
              ) {
                pauseTimer(notification);
              }
            }}
            onMouseLeave={() => {
              setNotifications(notification.id, "isMouseInside", false);

              if (
                notification.remainingDuration > 0 &&
                !notification.copied &&
                windowFocused()
              ) {
                resumeTimer(notification);
              }
            }}
          >
            <div
              class="absolute top-0 left-0 h-1 w-full"
              classList={{
                "bg-red-400": notification.type === "error",
                "bg-yellow-400": notification.type === "warning",
                "bg-green-400": notification.type === "success",
              }}
              style={{
                "border-radius": "0.5rem 0.5rem 0 0",
                width: `${
                  (notification.remainingDuration / notification.duration) * 100
                }%`,
              }}
            />

            <div
              class="absolute left-4 top-4 min-w-6 min-h-6 w-6 h-6 text-darkSlate-300 hover:text-lightSlate-100 duration-100 ease-in-out"
              classList={{
                "i-ri:arrow-up-s-line": !notification.expanded,
                "i-ri:arrow-down-s-line": notification.expanded,
              }}
              onClick={() => {
                setNotifications(notification.id, "expanded", (prev) => !prev);
              }}
            />
            <div class="min-w-6 min-h-6 w-6 h-6 i-ri:check-fill text-green-400" />
            <div class="w-auto h-full overflow-y-auto mx-4">
              <div class="font-bold text-lg">{notification.name}</div>
              <div class="text-md">{notification.name}</div>
            </div>
            <div class="min-h-full h-full flex flex-row gap-4">
              <div class="min-w-[1px] w-[1px] h-full bg-darkSlate-700" />
              <div class="flex flex-col justify-between">
                <div
                  class="w-6 h-6 text-darkSlate-300 i-ri:close-fill hover:text-lightSlate-100 duration-100 ease-in-out"
                  onClick={() => {
                    removeNotification(notification.id);
                  }}
                />
                <Tooltip content={"Copy"}>
                  <div
                    class="w-6 h-6"
                    classList={{
                      "text-darkSlate-300 hover:text-lightSlate-100 duration-100 ease-in-out i-ri:file-copy-2-fill":
                        !notification.copied,
                      "text-green-400 i-ri:checkbox-circle-fill":
                        notification.copied,
                    }}
                    onClick={() => {
                      navigator.clipboard.writeText(notification.name);
                      setNotifications(notification.id, "copied", true);
                      pauseTimer(notification);
                      setTimeout(() => {
                        setNotifications(notification.id, "copied", false);

                        if (!notification.isMouseInside && windowFocused()) {
                          resumeTimer(notification);
                        }
                      }, 2000);
                    }}
                  />
                </Tooltip>
              </div>
            </div>
          </div>
        )}
      </For>
      {props.children}
    </NotificationContext.Provider>
  );
};

const createNotification = () => {
  const context = useContext(NotificationContext);
  if (context === undefined) {
    throw new Error(
      "`createNotification` must be used within a `NotificationsProvider`."
    );
  }
  return context;
};

export { NotificationsProvider, createNotification };
