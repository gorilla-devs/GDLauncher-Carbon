import { Skeleton } from "@gd/ui"
import { Match, Switch, JSX } from "solid-js"
import { Trans } from "@gd/i18n"

interface RouteData {
  isLoading: boolean
  isError: boolean
  isSuccess: boolean
}

interface Props {
  routeData: { data: RouteData }
  children?: JSX.Element
}

const LoadingError = (props: Props) => {
  // TODO: show the correct error
  return (
    <Switch>
      <Match when={props.routeData.data.isLoading}>
        <Skeleton.contentArea />
      </Match>
      <Match when={props.routeData.data.isError}>
        <div class="flex h-full w-full items-center justify-center">
          <Trans key="errors:_trn_some_error" />
        </div>
      </Match>
      <Match when={props.routeData.data.isSuccess}>{props.children}</Match>
    </Switch>
  )
}

export default LoadingError
