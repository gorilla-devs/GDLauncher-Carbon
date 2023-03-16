import fetchData from "@/pages/Login/auth.login.data";
import { Navigate, useRouteData } from "@solidjs/router";
import { Show, Component } from "solid-js";
import { Dynamic } from "solid-js/web";

type Props = {
  component: Component;
};

export const PrivateRoute = (props: Props) => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();
  const isAlreadyAuthenticated = () =>
    routeData?.activeUuid?.data && routeData.accounts?.data?.length! > 0;

  return (
    <Show
      when={isAlreadyAuthenticated()}
      keyed
      fallback={<Navigate href={"/"} />}
    >
      <Dynamic component={props.component} />
    </Show>
  );
};
