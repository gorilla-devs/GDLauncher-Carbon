import { For, onCleanup } from "solid-js";
import { createStore } from "solid-js/store";

type Notification = {
  name: string;
  type?: string;
  position?: string;
};

const [notifications, setNotifications] = createStore<Notification[]>([]);
const clearNotification = () =>
  setTimeout(
    () =>
      setNotifications((notification) =>
        notification.slice(1, notification.length)
      ),
    2000
  );

onCleanup(() => clearInterval(clearNotification()));

export const addNotification = (
  name: string,
  type?: string,
  position?: string
) => {
  setNotifications((prev) => [
    ...prev,
    {
      name,
      // type can be success, error or warning and it change the style/color of the notification
      type: type || "success",
      // position can be bottom or top, if not present, by default is bottom
      position: position || "bottom",
    },
  ]);
  clearNotification();
};

const Notifications = () => {
  return (
    <div>
      <For each={notifications}>
        {(notification, i) => (
          <div
            class="w-50 h-10 px-4 text-white fixed left-1/2 rounded-md flex justify-center items-center"
            style={{
              transform: `translate(-50%, ${
                notification.position === "bottom"
                  ? `-${i() * 45}`
                  : `${i() * 45}`
              }px)`,
              transition: "transform 1s",
            }}
            classList={{
              "bottom-10": notification.position === "bottom",
              "bottom-auto": notification.position !== "bottom",
              "top-12": notification.position === "top",
              "top-auto": notification.position !== "top",
              "bg-status-red": notification.type === "error",
              "bg-status-yellow": notification.type === "warning",
              "bg-status-green": notification.type === "success",
            }}
          >
            {notification.name}
          </div>
        )}
      </For>
    </div>
  );
};

export { Notifications };
