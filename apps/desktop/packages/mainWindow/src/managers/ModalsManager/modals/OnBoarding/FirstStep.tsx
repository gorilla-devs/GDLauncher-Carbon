import { rspc } from "@/utils/rspcClient";
import { Trans } from "@gd/i18n";
import { Button } from "@gd/ui";

type Props = {
  nextStep: () => void;
};

const FirstStep = (props: Props) => {
  const accounts = rspc.createQuery(() => ["account.getAccounts"]);
  const activeUuid = rspc.createQuery(() => ["account.getActiveUuid"]);

  const currentAccount = accounts.data?.find(
    (account) => account.uuid === activeUuid.data
  );

  return (
    <div class="flex flex-col items-center justify-around box-border lg:w-160 lg:h-140">
      <div class="flex flex-col lg:mt-12 mt-6 lg:w-[35rem]">
        <h1 class="text-center text-lg m-0 font-bold mt-10">
          <Trans
            key="onboarding.welcome_gdlauncher_title"
            options={{
              defaultValue: "Welcome to GDLauncher",
            }}
          />
        </h1>
        <pre class="text-darkSlate-100 text-left leading-6 max-w-140 whitespace-pre-line text-xs lg:text-base">
          <Trans
            key="onboarding.welcome_gdlauncher_text"
            options={{
              user: currentAccount?.username,
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
