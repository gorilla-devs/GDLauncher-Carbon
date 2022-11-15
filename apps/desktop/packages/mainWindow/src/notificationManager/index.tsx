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

const Notifications = () => {
  return (
    <div>
      <For each={notifications}>
        {(notification, i) => (
          <div
            class="w-50 h-10 bg-light-100 text-black fixed bottom-5 left-1/2 rounded-md"
            style={{
              transform: `translate(-50%, -${i() * 43}px)`,
              transition: "transform 1s",
            }}
          >
            {notification.name}
          </div>
        )}
      </For>
    </div>
  );
};

export default Notifications;
