import { Trans } from "@gd/i18n";
import { Button, Input } from "@gd/ui";
import { rspc } from "@/utils/rspcClient";
import PageTitle from "./components/PageTitle";
import Row from "./components/Row";
import Title from "./components/Title";
import RowsContainer from "./components/RowsContainer";
import { useGlobalStore } from "@/components/GlobalStoreContext";
import { Match, Show, Switch } from "solid-js";
import { useGDNavigate } from "@/managers/NavigationManager";

const Accounts = () => {
  const globalStore = useGlobalStore();

  const navigate = useGDNavigate();

  const settingsMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.setSettings"]
  }));

  const removeGDLAccountMutation = rspc.createMutation(() => ({
    mutationKey: ["account.removeGdlAccount"]
  }));

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
          <div class="bg-darkSlate-700 p-4">
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
                      REMOVE
                    </Button>
                  </div>
                  <Show when={!globalStore?.gdlAccount.data?.isEmailVerified}>
                    <div class="bg-yellow-700 p-4 text-white rounded-md">
                      <Trans key="settings:gdl_account_not_verified" />
                    </div>
                  </Show>
                  <RowsContainer>
                    <Row>
                      <Title>
                        <Trans key="settings:linked_to_microsoft_account_username" />
                      </Title>
                      <div>
                        {
                          globalStore.accounts.data?.find(
                            (account) =>
                              account.uuid ===
                              globalStore.settings.data?.gdlAccountId
                          )?.username
                        }
                      </div>
                    </Row>
                    <Row>
                      <Title>
                        <Trans key="settings:linked_to_microsoft_account_id" />
                      </Title>
                      <div>{globalStore?.gdlAccount.data?.microsoftOid}</div>
                    </Row>
                    <Row>
                      <Title>
                        <Trans key="settings:gdl_account_recovery_email" />
                      </Title>
                      <div>{globalStore?.gdlAccount.data?.email}</div>
                    </Row>
                  </RowsContainer>
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
