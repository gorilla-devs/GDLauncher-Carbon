import { useRouteData } from "@solidjs/router";
import { createEffect, createSignal, Show } from "solid-js";
import { Trans } from "@gd/i18n";
import { rspc } from "@/utils/rspcClient";
import { Button, Collapsable, Dropdown } from "@gd/ui";
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
      <div class="w-full flex justify-center items-center">
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
      <div class="flex-1 px-4">
        <h2>FAQs</h2>
        <Collapsable
          defaultOpened={false}
          title="What is a GDLauncher account?"
        >
          <p class="text-lightSlate-500 text-md">
            A GDLauncher account is a way to save your settings, preferences,
            instance backups and more across all your devices.
          </p>
        </Collapsable>
        <Collapsable defaultOpened={false} title="How does it work?">
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
        </Collapsable>
        <Collapsable
          defaultOpened={false}
          title="What if I lose access to my Microsoft account?"
        >
          <p class="text-lightSlate-500 text-md">
            Since we don't store any info about you, the only way to recover
            your data in case you lose access to your Microsoft account is to
            use a different recovery email (in the next step).
          </p>
        </Collapsable>
        <Collapsable
          defaultOpened={false}
          title="What happens if I skip the account creation?"
        >
          <p class="text-lightSlate-500 text-md">
            The GDL account creation is completely optional. If you do decide to
            not create one, some features of the launcher might be unavailable.
          </p>
        </Collapsable>
      </div>
    </div>
  );
};

export default GDLAccount;
