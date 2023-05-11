import { useRouteData } from "@solidjs/router";
import fetchData from "../modpack.data";
import { createEffect } from "solid-js";

/* eslint-disable i18next/no-literal-string */
const Versions = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  createEffect(() => {
    console.log("routeData", routeData.modpackDetails.data?.data);
  });
  return <div>Versions</div>;
};

export default Versions;
