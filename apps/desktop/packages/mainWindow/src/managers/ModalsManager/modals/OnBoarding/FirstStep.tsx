import { rspc } from "@/utils/rspcClient"
import { Trans } from "@gd/i18n"
import { Button } from "@gd/ui"

interface Props {
  nextStep: () => void
}

const FirstStep = (props: Props) => {
  const accounts = rspc.createQuery(() => ({
    queryKey: ["account.getAccounts"]
  }))
  const activeUuid = rspc.createQuery(() => ({
    queryKey: ["account.getActiveUuid"]
  }))

  const currentAccount = accounts.data?.find(
    (account) => account.uuid === activeUuid.data
  )

  return (
    <div class="lg:w-160 box-border flex h-full flex-col justify-between">
      <div class="flex flex-col lg:w-[35rem]">
        <h1 class="m-0 mt-10 text-center text-lg font-bold">
          <Trans key="onboarding:_trn_welcome_gdlauncher_title" />
        </h1>
        <pre class="text-lightSlate-700 max-w-140 whitespace-pre-line text-left text-xs leading-5 lg:text-base">
          <Trans
            key="onboarding:_trn_welcome_gdlauncher_text"
            options={{ user: currentAccount?.username || "there" }}
          />
        </pre>
      </div>

      <div class="flex w-full justify-end">
        <Button
          type="primary"
          size="large"
          onClick={() => {
            props.nextStep()
          }}
        >
          <Trans key="onboarding:_trn_next" />
        </Button>
      </div>
    </div>
  )
}

export default FirstStep
