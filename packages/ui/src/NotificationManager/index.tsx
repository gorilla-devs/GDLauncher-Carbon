import { createContext, createEffect, For, JSX, useContext } from "solid-js";
import { createStore, produce } from "solid-js/store";

type NotificationType = "success" | "warning" | "error";
type NotificationPosition = "bottom" | "top";

type Notification = {
  id: number;
  name: string;
  type?: NotificationType;
  position?: NotificationPosition;
  duration: number;
  remainingDuration: number;
  expanded: boolean;
  timerId?: number;
  progressIntervalId?: number;
  copied: boolean;
  isMouseInside: boolean;
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

  createEffect(() => {
    const onFocus = () => {
      for (const notification of Object.values(notifications)) {
        if (notification.isMouseInside) continue;

        if (notification.remainingDuration > 0) {
          resumeTimer(notification);
        }
      }
    };
    const onBlur = () => {
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
    };

    newNotification.progressIntervalId = window.setInterval(() => {
      setNotifications(id, "remainingDuration", (prev) =>
        Math.max(prev - 16, 0)
      );
    }, 16);

    newNotification.timerId = window.setTimeout(() => {
      setNotifications(produce((prev) => delete prev[id]));
      if (newNotification.progressIntervalId) {
        clearInterval(newNotification.progressIntervalId);
      }
    }, duration);

    setNotifications(id, newNotification);
  };

  const pauseTimer = (notification: Notification) => {
    if (notification.timerId) clearTimeout(notification.timerId);
    if (notification.progressIntervalId)
      clearInterval(notification.progressIntervalId);
    notification.timerId = undefined;
    notification.progressIntervalId = undefined;
  };

  const resumeTimer = (notification: Notification) => {
    const newProgressIntervalId = window.setInterval(() => {
      setNotifications(notification.id, "remainingDuration", (prev) =>
        Math.max(prev - 16, 0)
      );
    }, 16);

    const newTimerId = window.setTimeout(() => {
      setNotifications(produce((prev) => delete prev[notification.id]));
      if (newProgressIntervalId) {
        clearInterval(newProgressIntervalId);
      }
    }, notification.remainingDuration);

    setNotifications(notification.id, "timerId", newTimerId);
    setNotifications(
      notification.id,
      "progressIntervalId",
      newProgressIntervalId
    );
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

              if (notification.remainingDuration > 0 && !notification.copied) {
                pauseTimer(notification);
              }
            }}
            onMouseLeave={() => {
              setNotifications(notification.id, "isMouseInside", false);

              if (notification.remainingDuration > 0 && !notification.copied) {
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
                    pauseTimer(notification);
                    setNotifications(
                      produce((prev) => delete prev[notification.id])
                    );
                  }}
                />
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

                      if (!notification.isMouseInside) {
                        resumeTimer(notification);
                      }
                    }, 2000);
                  }}
                />
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
