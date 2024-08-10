import { createEffect, Show, Suspense } from "solid-js";
import { Trans } from "@gd/i18n";
import { rspc } from "@/utils/rspcClient";
import { Collapsable } from "@gd/ui";

type Props = {
  activeUuid: string | null | undefined;
};

const GDLAccount = (props: Props) => {
  const accounts = rspc.createQuery(() => ({
    queryKey: ["account.getAccounts"]
  }));

  const currentlySelectedAccount = () =>
    accounts.data?.find((v) => v.uuid === props.activeUuid);

  const gdlUser = rspc.createQuery(() => ({
    queryKey: ["account.getGdlAccount", props.activeUuid!],
    enabled: !!props.activeUuid
  }));

  const currentlySelectedAccountEmail = () => {
    const account = currentlySelectedAccount();

    if (!account) return "";

    const email =
      account.type.type === "microsoft"
        ? account.type.value.email
        : account.username;

    return " - " + email;
  };

  createEffect(() => {
    if (props.activeUuid) {
      gdlUser.refetch();
    }
  });

  return (
    <Suspense>
      <div class="flex flex-col h-full w-full text-center">
        <Show when={gdlUser.data}>
          <div class="flex-1 px-4">
            <h2>
              <Trans
                key="login.welcome_back_name"
                options={{
                  name: currentlySelectedAccount()?.username
                }}
              />
            </h2>
            <p class="text-lightSlate-500 text-md">
              <Trans key="login.gdlauncher_account_description" />
            </p>
          </div>
        </Show>

        <Show when={!gdlUser.data}>
          <div class="flex-1 px-4">
            <h2>
              <Trans key="login.faqs" />
            </h2>
            <Collapsable
              defaultOpened={false}
              title={<Trans key="login.what_is_a_gdlauncher_account" />}
            >
              <p class="text-lightSlate-500 text-md">
                <Trans key="login.what_is_a_gdlauncher_account_text" />
              </p>
            </Collapsable>
            <Collapsable
              defaultOpened={false}
              title={<Trans key="login.how_does_it_work" />}
            >
              <p class="text-lightSlate-500 text-md">
                <Trans
                  key="login.how_does_it_work_text"
                  options={{
                    account_id: `${currentlySelectedAccount()?.username}${currentlySelectedAccountEmail()}`
                  }}
                >
                  <span class="text-white font-bold" />
                </Trans>
              </p>
            </Collapsable>
            <Collapsable
              defaultOpened={false}
              title={
                <Trans key="login.what_if_i_lose_access_to_my_microsoft_account" />
              }
            >
              <p class="text-lightSlate-500 text-md">
                <Trans key="login.what_if_i_lose_access_to_my_microsoft_account_text" />
              </p>
            </Collapsable>
            <Collapsable
              defaultOpened={false}
              title={
                <Trans key="login.what_happens_if_i_skip_the_account_creation" />
              }
            >
              <p class="text-lightSlate-500 text-md">
                <Trans key="login.what_happens_if_i_skip_the_account_creation_text" />
              </p>
            </Collapsable>
          </div>
        </Show>
      </div>
    </Suspense>
  );
};

export default GDLAccount;
