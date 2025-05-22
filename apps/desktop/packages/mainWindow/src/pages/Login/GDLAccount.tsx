import { createEffect, Match, Show, Suspense, Switch } from "solid-js"
import { Trans, useTransContext } from "@gd/i18n"
import { port, rspc } from "@/utils/rspcClient"
import { Collapsable, Dropdown } from "@gd/ui"

interface Props {
  activeUuid: string | null | undefined
}

const GDLAccount = (props: Props) => {
  const [t] = useTransContext()

  const setActiveAccountMutation = rspc.createMutation(() => ({
    mutationKey: ["account.setActiveUuid"]
  }))

  const accounts = rspc.createQuery(() => ({
    queryKey: ["account.getAccounts"]
  }))

  const currentlySelectedAccount = () =>
    accounts.data?.find((v) => v.uuid === props.activeUuid)

  const gdlUser = rspc.createQuery(() => ({
    queryKey: ["account.peekGdlAccount", props.activeUuid!],
    enabled: !!props.activeUuid
  }))

  const currentlySelectedAccountEmail = () => {
    const account = currentlySelectedAccount()

    if (!account) return ""

    const email =
      account.type.type === "microsoft"
        ? account.type.value.email
        : account.username

    return " - " + email
  }

  createEffect(() => {
    if (props.activeUuid) {
      gdlUser.refetch()
    }
  })

  const accountOptions = () => {
    return accounts.data?.map((account) => {
      return {
        label: (
          <div
            class="flex justify-between items-center gap-4"
            onClick={() => {
              setActiveAccountMutation.mutate(account.uuid)
            }}
          >
            <div class="flex items-center gap-4">
              <img
                src={`http://127.0.0.1:${port}/account/headImage?uuid=${account.uuid}`}
                class="w-6 h-6 rounded-md"
              />
              <div class="truncate max-w-30">{account.username}</div>
            </div>
            <div class="flex items-center gap-2">
              <Switch>
                <Match when={account.type.type === "microsoft"}>
                  <div class="w-4 h-4 i-ri:microsoft-fill" />
                </Match>
                <Match when={account.type.type === "offline"}>
                  <div class="w-4 h-4 i-ri:computer-line" />
                </Match>
              </Switch>
              <Switch>
                <Match when={account.status === "ok"}>
                  <div class="w-4 h-4 text-green-500 i-ri:check-fill" />
                </Match>
                <Match when={account.status === "expired"}>
                  <div class="w-4 h-4 text-yellow-500 i-ri:alert-fill" />
                </Match>
                <Match when={account.status === "refreshing"}>
                  <div class="w-4 h-4 text-yellow-500 i-ri:loader-4-fill" />
                </Match>
                <Match when={account.status === "invalid"}>
                  <div class="w-4 h-4 text-red-500 i-ri:close-fill" />
                </Match>
              </Switch>
            </div>
          </div>
        ),
        key: account?.uuid
      }
    })
  }

  return (
    <Suspense>
      <div class="flex flex-col h-full w-full text-center pt-2 box-border">
        <div class="flex items-center justify-center gap-4">
          <div>
            <Trans key="login.link_account" />
          </div>
          <Dropdown
            options={accountOptions() || []}
            value={currentlySelectedAccount()?.uuid}
          />
        </div>
        <Show when={gdlUser.data}>
          <div class="flex-1 p-4">
            <div class="text-2xl font-bold">
              <Trans
                key="login.welcome_back_name"
                options={{
                  name: currentlySelectedAccount()?.username
                }}
              />
            </div>
            <p class="text-lightSlate-700 text-md pt-2">
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
              title={t("login.what_is_a_gdlauncher_account")}
            >
              <p class="text-lightSlate-700 text-md">
                <Trans key="login.what_is_a_gdlauncher_account_text" />
              </p>
            </Collapsable>
            <Collapsable
              defaultOpened={false}
              title={t("login.how_does_it_work")}
            >
              <p class="text-lightSlate-700 text-md">
                <Trans
                  key="login.how_does_it_work_text"
                  options={{
                    account_id: `${currentlySelectedAccount()?.username}${currentlySelectedAccountEmail()}`
                  }}
                >
                  {""}
                  <span class="text-lightSlate-50 font-bold" />
                  {""}
                </Trans>
              </p>
            </Collapsable>
            <Collapsable
              defaultOpened={false}
              title={t("login.what_if_i_lose_access_to_my_microsoft_account")}
            >
              <p class="text-lightSlate-700 text-md">
                <Trans key="login.what_if_i_lose_access_to_my_microsoft_account_text" />
              </p>
            </Collapsable>
            <Collapsable
              defaultOpened={false}
              title={t("login.what_happens_if_i_skip_the_account_creation")}
            >
              <p class="text-lightSlate-700 text-md">
                <Trans key="login.what_happens_if_i_skip_the_account_creation_text" />
              </p>
            </Collapsable>
          </div>
        </Show>
      </div>
    </Suspense>
  )
}

export default GDLAccount
