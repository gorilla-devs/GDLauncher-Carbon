import { useRouteData } from "@solidjs/router";
import { createEffect, createSignal, Show, Suspense } from "solid-js";
import { Trans } from "@gd/i18n";
import { rspc } from "@/utils/rspcClient";
import { Button, Collapsable, Dropdown } from "@gd/ui";
import fetchData from "./auth.login.data";

type Props = {
  activeUuid: string | null | undefined;
};

const GDLAccount = (props: Props) => {
  const accounts = rspc.createQuery(() => ({
    queryKey: ["account.getAccounts"]
  }));

  const currentlySelectedAccount = () =>
    accounts.data?.find((v) => v.uuid === props.activeUuid);

  return (
    <Suspense>
      <div class="flex flex-col h-full w-full text-center">
        <h3 class="text-lightSlate-300">
          Welcome Back {currentlySelectedAccount()?.username}
        </h3>

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
              A GDLauncher account is just an entry in our database that is
              linked to your Microsoft account ID (
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
              The GDL account creation is completely optional. If you do decide
              to not create one, some features of the launcher might be
              unavailable.
            </p>
          </Collapsable>
        </div>
      </div>
    </Suspense>
  );
};

export default GDLAccount;
