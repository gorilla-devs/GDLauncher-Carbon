import { Trans, useTransContext } from "@gd/i18n";
import { Button, Input, Tooltip } from "@gd/ui";
import { port, rspc } from "@/utils/rspcClient";
import PageTitle from "./components/PageTitle";
import Row from "./components/Row";
import Title from "./components/Title";
import RowsContainer from "./components/RowsContainer";
import { useGlobalStore } from "@/components/GlobalStoreContext";
import { createEffect, JSX, Match, Show, Switch } from "solid-js";
import { useGDNavigate } from "@/managers/NavigationManager";
import { convertSecondsToHumanTime } from "@/utils/helpers";
import { useModal } from "@/managers/ModalsManager";

const GDLAccountRow = (props: { children: JSX.Element }) => {
  return (
    <div class="flex items-center justify-between gap-4">{props.children}</div>
  );
};

const GDLAccountRowItem = (props: {
  title?: string;
  value?: string | null | undefined;
  children?: JSX.Element;
  onEdit?: () => void;
}) => {
  return (
    <div class="flex justify-between items-center">
      <div class="flex flex-col gap-2 justify-center">
        <Show when={props.title}>
          <div class="text-md font-bold text-lightSlate-600 uppercase">
            {props.title}
          </div>
        </Show>
        <Show when={props.value}>
          <div class="text-lightSlate-50 text-ellipsis overflow-hidden whitespace-nowrap">
            {props.value}
          </div>
        </Show>
        {props.children}
      </div>
      <Show when={props.onEdit}>
        <div class="text-md underline">EDIT</div>
      </Show>
    </div>
  );
};

const Accounts = () => {
  const globalStore = useGlobalStore();
  const [t] = useTransContext();

  const navigate = useGDNavigate();
  const modalsContext = useModal();

  const removeGDLAccountMutation = rspc.createMutation(() => ({
    mutationKey: ["account.removeGdlAccount"]
  }));

  const requestNewVerificationTokenMutation = rspc.createMutation(() => ({
    mutationKey: ["account.requestNewVerificationToken"]
  }));

  createEffect(() => {
    console.log(globalStore.gdlAccount.data);
  });

  const deleteAccountContent = () => {
    if (globalStore.gdlAccount.data?.deletionTimeout) {
      return (
        <Trans
          key="settings:cannot_request_deletion_for_time"
          options={{
            time: convertSecondsToHumanTime(
              globalStore.gdlAccount.data?.deletionTimeout!
            )
          }}
        />
      );
    } else {
      return undefined;
    }
  };

  const verificationContent = () => {
    if (globalStore.gdlAccount.data?.verificationTimeout) {
      return (
        <Trans
          key="settings:cannot_request_deletion_for_time"
          options={{
            time: convertSecondsToHumanTime(
              globalStore.gdlAccount.data?.verificationTimeout!
            )
          }}
        />
      );
    } else {
      return undefined;
    }
  };

  return (
    <>
      <PageTitle>
        <Trans key="settings:accounts" />
      </PageTitle>
      <RowsContainer>
        <Row forceContentBelow>
          <Title>
            <Trans key="settings:gdl_account_title" />
          </Title>
          <div class="bg-darkSlate-700 p-4 mb-6">
            <Switch>
              <Match when={globalStore?.gdlAccount.data}>
                <div class="flex flex-col gap-4">
                  <div class="flex gap-2 items-center justify-between">
                    <div class="text-green-400 text-xl">
                      <Trans key="settings:gdl_account_synced" />
                    </div>

                    <Button
                      type="outline"
                      onClick={() => {
                        removeGDLAccountMutation.mutate(undefined);
                      }}
                    >
                      LOG OUT
                    </Button>
                  </div>
                  <Show when={!globalStore?.gdlAccount.data?.isEmailVerified}>
                    <div class="flex items-center justify-between outline outline-yellow-500 text-yellow-500 p-4 rounded-md mb-4">
                      <div class="flex items-center gap-4">
                        <i class="block w-6 h-6 i-ri:alert-fill" />
                        <Trans key="settings:gdl_account_not_verified" />
                      </div>
                      <Tooltip content={verificationContent()}>
                        <Button
                          disabled={
                            !!globalStore.gdlAccount.data?.verificationTimeout
                          }
                          onClick={async () => {
                            const uuid = globalStore.accounts.data?.find(
                              (account) =>
                                account.uuid ===
                                globalStore.settings.data?.gdlAccountId
                            )?.uuid;

                            if (!uuid) {
                              throw new Error("No active gdl account");
                            }

                            const request =
                              await requestNewVerificationTokenMutation.mutateAsync(
                                uuid
                              );

                            if (request.status === "failed" && request.value) {
                              throw new Error(
                                `Too many requests, retry in ${request.value}s`
                              );
                            }
                          }}
                        >
                          <Trans key="settings:send_new_verification_email" />
                        </Button>
                      </Tooltip>
                    </div>
                  </Show>
                  <div class="grid grid-cols-2 gap-4">
                    <GDLAccountRowItem
                      title={t("settings:minecraft_uuid")}
                      value={
                        globalStore.accounts.data?.find(
                          (account) =>
                            account.uuid ===
                            globalStore.settings.data?.gdlAccountId
                        )?.uuid
                      }
                    />
                    <GDLAccountRowItem
                      title={t("settings:microsoft_username")}
                      value={
                        globalStore.accounts.data?.find(
                          (account) =>
                            account.uuid ===
                            globalStore.settings.data?.gdlAccountId
                        )?.username
                      }
                    />
                    <GDLAccountRowItem
                      title={t("settings:recovery_email")}
                      value={globalStore.gdlAccount.data?.email}
                      onEdit={() => {}}
                    />
                    <GDLAccountRowItem
                      title={t("settings:microsoft_email")}
                      value={globalStore.gdlAccount.data?.microsoftEmail}
                    />
                  </div>
                </div>

                <div class="my-10 text-red-500 text-xl">
                  <Trans key="settings:danger_zone" />
                </div>
                <div class="flex items-center justify-between gap-12 text-lightSlate-800">
                  <div>
                    <Trans key="settings:request_account_deletion_description" />
                  </div>
                  <Tooltip content={deleteAccountContent()}>
                    <Button
                      variant="red"
                      size="large"
                      disabled={!!globalStore.gdlAccount.data?.deletionTimeout}
                      onClick={() => {
                        modalsContext?.openModal({
                          name: "confirmGDLAccountDeletion"
                        });
                      }}
                    >
                      <Trans key="settings:request_account_deletion" />
                    </Button>
                  </Tooltip>
                </div>
              </Match>
              <Match when={!globalStore?.gdlAccount.data}>
                <div class="flex gap-2 items-center justify-between">
                  <div class="text-red-400 text-xl">
                    <Trans key="settings:gdl_account_not_synced" />
                  </div>

                  <Button
                    type="outline"
                    onClick={async () => {
                      await removeGDLAccountMutation.mutateAsync(undefined);
                      navigate("/");
                    }}
                  >
                    ADD GDL ACCOUNT
                  </Button>
                </div>
              </Match>
            </Switch>
          </div>
        </Row>
      </RowsContainer>
    </>
  );
};

export default Accounts;
