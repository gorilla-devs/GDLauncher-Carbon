import { useGDNavigate } from "@/managers/NavigationManager";
import fetchData from "@/pages/Login/auth.login.data";
import { useRouteData } from "@solidjs/router";
import { Show, Component, createEffect } from "solid-js";
import { Dynamic } from "solid-js/web";

type Props = {
  component: Component;
};

const PrivateRoute = (props: Props) => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();
  const navigate = useGDNavigate();
  const isAlreadyAuthenticated = () =>
    routeData?.activeUuid?.data &&
    routeData.accounts.data?.length! > 0 &&
    !routeData.status.data &&
    routeData.settings.data?.termsAndPrivacyAccepted;

  createEffect(() => {
    console.log(isAlreadyAuthenticated());
    if (!isAlreadyAuthenticated()) navigate("/");
  });

  return (
    <Show when={isAlreadyAuthenticated()} keyed>
      <Dynamic component={props.component} />
    </Show>
  );
};

export default PrivateRoute;
