import { rspc } from "@/utils/rspcClient";
import type { RouteDataFunc } from "@solidjs/router";

const SettingsAppearance: RouteDataFunc = () => {
  let data = rspc.createQuery(() => ["app.getTheme", null]);
  return { data };
};

export default SettingsAppearance;
