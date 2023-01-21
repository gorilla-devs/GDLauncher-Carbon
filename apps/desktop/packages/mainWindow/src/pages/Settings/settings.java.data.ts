import { rspc } from "@/utils/rspcClient";
import type { RouteDataFunc } from "@solidjs/router";

const SettingsJavaData: RouteDataFunc = () => {
  let data = rspc.createQuery(() => ["java.getAvailable", null]);
  return { data };
};

export default SettingsJavaData;
