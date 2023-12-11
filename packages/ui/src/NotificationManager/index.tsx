import {
  createContext,
  createEffect,
  createSignal,
  For,
  JSX,
  onCleanup,
  useContext,
} from "solid-js";

type NotificationType = "success" | "warning" | "error";
type NotificationPosition = "bottom" | "top";

type Notification = {
  name: string;
  type?: NotificationType;
  position?: string;
  createdAt: number;
  duration: number;
};

type Props = {
  children: Element | HTMLElement | JSX.Element | any;
};

type NotificationContextType = (
  _name: string,
  _type?: NotificationType,
  _position?: NotificationPosition,
  _duration?: number
) => void;

const NotificationContext = createContext<NotificationContextType>();

const NotificationsProvider = (props: Props) => {
  const [notifications, setNotifications] = createSignal<Notification[]>([]);

  const addNotification = (
    name: string,
    type?: NotificationType,
    position?: NotificationPosition,
    duration: number = 2000
  ) => {
    const createdAt = Date.now();
    const newNotification = {
      name,
      // type can be success, error or warning and it change the style/color of the notification
      type: type || "success",
      // position can be bottom or top, if not present, by default is bottom
      position: position || "bottom",
      createdAt,
      duration,
    };

    setNotifications((prev) => [...prev, newNotification]);
  };

  const setTimers = () => {
    const now = Date.now();
    const timers = notifications().map((notification) => {
      const durationLeft =
        notification.duration - (now - notification.createdAt);
      if (durationLeft <= 0) {
        setNotifications((prev) => {
          return prev.filter((n) => n.createdAt !== notification.createdAt);
        });
        return;
      }

      return setTimeout(() => {
        setNotifications((prev) => {
          return prev.filter((n) => n.createdAt !== notification.createdAt);
        });
      }, durationLeft);
    });
    return timers;
  };

  createEffect(() => {
    const timers = setTimers();
    onCleanup(() => {
      if (!timers) return;
      timers.forEach((timer) => timer && clearTimeout(timer));
    });
  });

  return (
    <>
      <NotificationContext.Provider value={addNotification}>
        <For each={notifications()}>
          {(notification, i) => (
            <div
              class="w-50 h-10 px-4 text-white fixed left-1/2 rounded-md flex justify-center items-center z-100"
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
                "bg-red-200": notification.type === "error",
                "bg-yellow-200": notification.type === "warning",
                "bg-green-200": notification.type === "success",
              }}
            >
              <div
                class="i-ri:close-fill absolute top-1 right-1 text-white w-4 h-4 cursor-pointer"
                onClick={() => {
                  setNotifications((prev) => {
                    return prev.filter(
                      (n) => n.createdAt !== notification.createdAt
                    );
                  });
                }}
              />
              <span>{notification.name}</span>
            </div>
          )}
        </For>
        {props.children}
      </NotificationContext.Provider>
    </>
  );
};

const createNotification = () => {
  const context = useContext(NotificationContext);

  if (context === undefined) {
    throw new Error(
      "`createNotification` must be used within a `NotificationContext`"
    );
  }

  return context;
};

export { NotificationsProvider, createNotification };
