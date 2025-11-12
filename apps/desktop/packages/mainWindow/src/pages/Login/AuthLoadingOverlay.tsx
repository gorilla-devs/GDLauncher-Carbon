import { Show, createSignal, createMemo } from "solid-js"
import { Trans } from "@gd/i18n"
import { Progress } from "@gd/ui"
import PatternBackground from "@/components/PatternBackground"
import RiveAppWapper from "@/utils/RiveAppWrapper"
import GDAnimation from "@/gd_logo_animation.riv"
import { CoreModuleStatus } from "@gd/core_module/bindings"

interface AuthLoadingOverlayProps {
  progress: number
  status: CoreModuleStatus | null
  visible: boolean
}

const AuthLoadingOverlay = (props: AuthLoadingOverlayProps) => {
  const [startTime] = createSignal(Date.now())

  const statusText = createMemo(() => {
    switch (props.status) {
      case "VerifyingTermsAndPrivacy":
        return "Verifying Terms & Privacy..."
      case "LoadAndMigrate":
        return "Initializing database..."
      case "RefreshMSAuth":
        return "Refreshing authentication..."
      case "XboxAuth":
        return "Connecting to Xbox Live..."
      case "McLogin":
        return "Authenticating with Minecraft..."
      case "MCEntitlements":
        return "Checking game ownership..."
      case "McProfile":
        return "Loading your profile..."
      case "AccountRefreshComplete":
        return "Finalizing..."
      case "LaunchBackgroundTasks":
        return "Almost ready..."
      default:
        return "Loading..."
    }
  })

  return (
    <div class="absolute inset-0 z-20 flex items-center justify-center">
      <PatternBackground>
        <div class="flex flex-col items-center gap-16">
          <RiveAppWapper src={GDAnimation} width={400} height={400} />

          <div class="w-96 text-center">
            <p class="text-lightSlate-50 mb-4 text-lg font-medium">
              {statusText()}
            </p>
            <Progress
              value={props.progress}
              class="h-3"
              color="bg-primary-500"
            />
          </div>

          <Show when={Date.now() - startTime() > 5000}>
            <p class="text-lightSlate-400 max-w-md text-center text-base">
              <Trans key="auth:_trn_auth.loading.taking_longer" />
            </p>
          </Show>
        </div>
      </PatternBackground>
    </div>
  )
}

export default AuthLoadingOverlay
