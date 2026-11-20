import { useTransContext } from "@gd/i18n"
import type { FEJavaComponentType, FEJavaUsage } from "@gd/core_module/bindings"
import { For, Show } from "solid-js"
import { firstNames } from "@/utils/java/javaText"
import { profileUsers } from "@/pages/Settings/java/javaOverviewView"
import { ModalProps } from "../.."
import ModalLayout from "../../ModalLayout"
import ConfirmActions, {
  ConfirmCallbacks,
  createConfirm
} from "../../ConfirmActions"

export interface DeleteJavaConfirmData extends ConfirmCallbacks {
  vendor: string
  version: string
  /** A Java the user added is only taken off the list; its files stay. */
  source: FEJavaComponentType
  usedBy: FEJavaUsage[]
  /** Live: a game using the Java is running, as the row's disabled delete
   *  icon says. */
  runningGame: () => boolean
}

/** Asks before deleting a Java that instances use: they move to Automatic. */
const DeleteJavaConfirm = (props: ModalProps) => {
  const [t] = useTransContext()
  const data = () => props.data as DeleteJavaConfirmData
  const { busy, confirm, cancel } = createConfirm(data)
  // A game may start while the dialog is open.
  const blocked = () => data().runningGame()

  const removesFromList = () => data().source === "custom"
  // Profile users are counted under their profile, which loses its Java
  // and so uses Automatic too.
  const users = () => profileUsers(data().usedBy)
  const names = () => firstNames(users().instances)
  // Every instance that moves, its profile's users included.
  const movingCount = () =>
    users().instances.length +
    users().profiles.reduce((sum, profile) => sum + profile.count, 0)
  const title = () =>
    t(
      removesFromList()
        ? "java:_trn_remove_java_title"
        : "java:_trn_delete_java_title",
      { vendor: data().vendor, version: data().version }
    )

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={title()}
      width="w-120"
      height="h-auto"
      preventClose={busy()}
      compact
    >
      <div class="flex flex-col gap-6">
        <div class="flex flex-col gap-4">
          {/* One list: the instances, then each profile with how many
              instances use it. */}
          <Show when={movingCount() > 0}>
            <div class="flex flex-col gap-2">
              <p class="text-lightSlate-700 m-0">
                {t("java:_trn_these_use_automatic", { count: movingCount() })}
              </p>
              <ul class="text-lightSlate-100 m-0 flex list-none flex-col gap-1 pl-4">
                <For each={names().shown}>
                  {(usage) => <li class="truncate">{usage.name}</li>}
                </For>
                <For each={users().profiles}>
                  {(profile) => (
                    <li class="flex min-w-0 items-baseline gap-1.5">
                      <span class="truncate">{profile.name}</span>
                      <span class="text-darkSlate-400">·</span>
                      <span class="text-lightSlate-700 shrink-0">
                        {t("java:_trn_instance_count", {
                          count: profile.count
                        })}
                      </span>
                    </li>
                  )}
                </For>
                <Show when={names().hidden > 0}>
                  <li class="text-lightSlate-700">
                    {t("java:_trn_and_more", { count: names().hidden })}
                  </li>
                </Show>
              </ul>
            </div>
          </Show>
          <Show when={blocked()}>
            <p class="m-0 text-yellow-500">{t("java:_trn_close_game_first")}</p>
          </Show>
        </div>
        <ConfirmActions
          busy={busy()}
          disabled={blocked()}
          onConfirm={() => {
            if (!blocked()) confirm()
          }}
          onCancel={cancel}
        >
          {removesFromList() ? t("java:_trn_remove") : t("general:_trn_delete")}
        </ConfirmActions>
      </div>
    </ModalLayout>
  )
}

export default DeleteJavaConfirm
