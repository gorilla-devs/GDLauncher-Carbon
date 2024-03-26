import { rspc } from "@/utils/rspcClient";
import { Trans } from "@gd/i18n";
import { Button } from "@gd/ui";

type Props = {
  nextStep: () => void;
};

const FirstStep = (props: Props) => {
  const accounts = rspc.createQuery(() => ({
    queryKey: ["account.getAccounts"]
  }));
  const activeUuid = rspc.createQuery(() => ({
    queryKey: ["account.getActiveUuid"]
  }));

  const currentAccount = accounts.data?.find(
    (account) => account.uuid === activeUuid.data
  );

  return (
    <div class="flex flex-col justify-between box-border h-full lg:w-160">
      <div class="flex flex-col lg:w-[35rem]">
        <h1 class="text-center text-lg m-0 font-bold mt-10">
          <Trans key="onboarding.welcome_gdlauncher_title" />
        </h1>
        <pre class="text-darkSlate-100 text-left text-xs max-w-140 whitespace-pre-line lg:text-base leading-5">
          <Trans
            key="onboarding.welcome_gdlauncher_text"
            options={{
              user: currentAccount?.username
            }}
          />
        </pre>
      </div>

      <div class="flex w-full justify-end">
        <Button
          type="primary"
          size="large"
          onClick={() => {
            props.nextStep();
          }}
        >
          <Trans key="onboarding.next" />
        </Button>
      </div>
    </div>
  );
};

export default FirstStep;
