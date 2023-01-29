/* eslint-disable i18next/no-literal-string */
import { Spinner } from "@gd/ui";
import { Match, Switch, JSX } from "solid-js";

interface RouteData {
  isLoading: boolean;
  isError: boolean;
  isSuccess: boolean;
}

interface Props {
  routeData: { data: RouteData };
  children?: JSX.Element;
}

const LoadingError = (props: Props) => {
  // TODO: show the correct error
  return (
    <Switch>
      <Match when={props.routeData.data.isLoading}>
        <div class="h-full w-full flex justify-center items-center">
          <Spinner />
        </div>
      </Match>
      <Match when={props.routeData.data.isError}>
        <div class="h-full w-full flex justify-center items-center">
          Some error
        </div>
      </Match>
      <Match when={props.routeData.data.isSuccess}>{props.children}</Match>
    </Switch>
  );
};

export default LoadingError;
