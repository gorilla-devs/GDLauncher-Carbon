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
    () => setNotifications((notification) => notification.slice(0, -1)),
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

const getColorByType = (type: string) => {
  switch (type) {
    case "success":
      return "#29A335";
    case "warning":
      return "#F7BC3D";
    case "error":
      return "#E54B4B";

    default:
      return "#29A335";
  }
};

const Notifications = () => {
  return (
    <div>
      <For each={notifications}>
        {(notification, i) => (
          <div
            class="w-50 h-10 bg-status-green text-black fixed left-1/2 rounded-md"
            style={{
              bottom: notification.position === "bottom" ? "20px" : "auto",
              top: notification.position === "top" ? "50px" : "auto",
              transform: `translate(-50%, ${
                notification.position === "bottom"
                  ? `-${i() * 43}`
                  : `${i() * 43}`
              }px)`,
              transition: "transform 1s",
              "background-color": getColorByType(notification.type || ""),
            }}
            // classList={{
            //   "bg-status-red": notification.type === "error",
            //   "bg-status-yellow": notification.type === "warning",
            //   "bg-status-green": notification.type === "success",
            // }}
          >
            {notification.name}
          </div>
        )}
      </For>
    </div>
  );
};

export default Notifications;
