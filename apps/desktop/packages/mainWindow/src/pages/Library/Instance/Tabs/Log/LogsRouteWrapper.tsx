import { Outlet } from "@solidjs/router";
import { isFullScreen } from ".";

const LogsRouteWrapper = () => {
  return (
    <div
      style={{
        height: isFullScreen() ? "calc(100vh - 135px)" : "calc(100vh - 389px)"
      }}
    >
      <Outlet />
    </div>
  );
};

export default LogsRouteWrapper;
