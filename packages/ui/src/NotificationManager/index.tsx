import {
  createContext,
  createSignal,
  For,
  JSX,
  onCleanup,
  useContext,
} from "solid-js";

type Notification = {
  name: string;
  type?: string;
  position?: string;
};

type Props = {
  children: Element | HTMLElement | JSX.Element | any;
};

const NotificationContext = createContext();

const NotificationsProvider = (props: Props) => {
  const [notifications, setNotifications] = createSignal<Notification[]>([]);
  const [notificationTimeOut, setNotificationTimeOut] = createSignal<
    ReturnType<typeof setTimeout>[]
  >([]);

  const addNotification = (name: string, type?: string, position?: string) => {
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

    setNotificationTimeOut((prev) => [
      ...prev,
      setTimeout(() => {
        setNotifications((notification) =>
          notification.slice(1, notification.length)
        );
      }, 2000),
    ]);
  };

  onCleanup(() => {
    for (const notificationTimeOutt of notificationTimeOut()) {
      clearTimeout(notificationTimeOutt);
    }
  });

  const value = [addNotification];

  return (
    <>
      <NotificationContext.Provider value={value}>
        <For each={notifications()}>
          {(notification, i) => (
            <div
              class="w-50 h-10 px-4 text-white fixed left-1/2 rounded-md flex justify-center items-center z-60"
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
        {props.children}
      </NotificationContext.Provider>
    </>
  );
};

const createNotification = () => {
  return (
    (useContext(NotificationContext) as [
      // eslint-disable-next-line no-unused-vars
      (name: string, type?: string, position?: string) => void
    ]) || []
  );
};

export { NotificationsProvider, createNotification };
