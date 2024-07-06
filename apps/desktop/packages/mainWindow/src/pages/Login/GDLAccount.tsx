import { useRouteData } from "@solidjs/router";
import { createEffect, createSignal, Show } from "solid-js";
import { Trans } from "@gd/i18n";
import { rspc } from "@/utils/rspcClient";
import { Button, Dropdown } from "@gd/ui";
import fetchData from "./auth.login.data";

const GDLAccount = () => {
  const [selectedAccount, setSelectedAccount] = createSignal<
    string | number | null
  >(null);

  const settingsMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.setSettings"]
  }));

  const accounts = rspc.createQuery(() => ({
    queryKey: ["account.getAccounts"]
  }));

  const currentlySelectedAccount = () =>
    accounts.data?.find((v) => v.uuid === selectedAccount());

  createEffect(() => {
    if (accounts.data?.length || 0 > 0) {
      console.log(accounts.data);
      setSelectedAccount(accounts.data?.[0].uuid || null);
    }
  });

  return (
    <div class="flex flex-col h-full w-full text-center">
      <h1>GDLauncher Account</h1>
      <div className="w-full flex justify-center items-center">
        <Dropdown
          class="w-full"
          options={accounts.data?.map((v) => ({
            label: `${v.username} (${v.type.type === "microsoft" ? v.type.value.email : v.username})`,
            key: v.uuid
          }))}
          value={selectedAccount()}
          onChange={(v) => setSelectedAccount(v.key)}
        />
      </div>
      <div class="flex-1 px-4 overflow-y-auto">
        <h3 class="text-center font-bold">What is a GDLauncher account?</h3>
        <p class="text-lightSlate-500 text-md">
          A GDLauncher account is a way to save your settings, preferences,
          instance backups and more across all your devices.
        </p>
        <h3 class="text-center font-bold">How does it work?</h3>
        <p class="text-lightSlate-500 text-md">
          A GDLauncher account is just an entry in our database that is linked
          to your Microsoft account ID (
          <span class="text-white font-bold">
            {currentlySelectedAccount()?.username}
            {" - "}
            {currentlySelectedAccount()?.type.type === "microsoft"
              ? currentlySelectedAccount()?.type.value.email
              : currentlySelectedAccount()?.username}
          </span>
          ). We use your token to authenticate you, your password is NEVER
          stored. (Learn more about how it works{" "}
          <a
            href="https://www.microsoft.com/en-us/security/business/security-101/what-is-oauth"
            target="_blank"
            class="text-primary-300"
          >
            here
          </a>
          )
        </p>
        <h3 class="text-center font-bold">
          What if I lose access to my Microsoft account?
        </h3>
        <p class="text-lightSlate-500 text-md">
          Since we don't store any info about you, there is currently no way to
          recover your GDLauncher account in case you lose access to your
          Microsoft account.
        </p>

        <div class="underline mt-8">See More Details</div>
      </div>

      <div class="flex justify-between items-center gap-4 w-auto p-4">
        <Button
          type="text"
          onClick={() => {
            settingsMutation.mutate({
              hasCompletedGdlAccountSetup: {
                Set: true
              }
            });
          }}
        >
          Skip
        </Button>
        <Button
          variant="primary"
          // disabled={!acceptedTOS()}
          // loading={loadingButton()}
          onClick={async () => {}}
        >
          Create Account
        </Button>
      </div>
    </div>
  );
};

export default GDLAccount;
