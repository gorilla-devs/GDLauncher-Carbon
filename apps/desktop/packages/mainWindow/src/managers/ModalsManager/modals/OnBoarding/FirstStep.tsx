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
    <div class="flex flex-col items-center justify-around w-160 h-140 box-border">
      <div class="flex flex-col mt-12">
        <h1 class="text-center text-lg m-0 font-bold mt-10">
          <Trans
            key="onboarding.welcome_gdlauncher_title"
            options={{
              defaultValue: "Welcome to GDLauncher",
            }}
          />
        </h1>
        <pre class="text-darkSlate-100 leading-6 text-left max-w-140 whitespace-pre-line">
          <Trans
            key="onboarding.welcome_gdlauncher_text"
            options={{
              defaultValue:
                "Hey {{user}}!\n\nWe're thrilled to have you onboard! GDLauncher is your personal Minecraft command center, designed to supercharge your gaming experience. \n\nFrom custom instances and integrated mod search to performance tweaking and design customization - everything you need is at your fingertips.\n\nWe recommend a quick tour of the app to get started. Our support team is always here to help.\n\nReady to craft your perfect Minecraft adventure? Let's get started!\n\nBest regards,\nThe GDLauncher Team",
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
