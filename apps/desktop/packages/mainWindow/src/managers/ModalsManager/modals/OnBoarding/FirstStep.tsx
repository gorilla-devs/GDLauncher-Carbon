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
    <div class="flex flex-col items-center justify-around w-160 h-140">
      <div class="flex flex-col mt-12">
        <h1 class="text-center text-lg m-0 mt-10 font-bold">
          <Trans
            key="onboarding.welcome_gdlauncher_title"
            options={{
              defaultValue: "Welcome to GDLauncher",
            }}
          />
        </h1>
        <pre class="text-darkSlate-100 text-left leading-6 mb-8 max-w-140 whitespace-pre-line">
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
      <div class="flex flex-col items-center gap-6 w-full">
        <Button
          style={{ width: "100%", "max-width": "200px" }}
          onClick={() => {
            props.nextStep();
          }}
        >
          <Trans
            key="onboarding.continue"
            options={{
              defaultValue: "Continue",
            }}
          />
        </Button>
      </div>
    </div>
  );
};

export default FirstStep;
