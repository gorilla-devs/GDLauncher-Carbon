import { createMemo, createSignal } from "solid-js"
import { Trans, useTransContext } from "@gd/i18n"
import {
  Input,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue
} from "@gd/ui"
import { rspc } from "@/utils/rspcClient"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import HookTest from "@/components/HookTest"
import PageTitle from "./components/PageTitle"
import Row from "./components/Row"
import Title from "./components/Title"
import RowsContainer from "./components/RowsContainer"

/** The instance a launcher-wide hook is tested in unless another is picked:
 * the one played last, or else the first. */
export function defaultTestInstance(
  instances: readonly { id: number; last_played: string | null }[]
): number | undefined {
  const played = instances
    .filter((instance) => instance.last_played !== null)
    .sort((a, b) => (a.last_played! < b.last_played! ? 1 : -1))
  return (played[0] ?? instances[0])?.id
}

const CustomCommands = () => {
  const [t] = useTransContext()
  const globalStore = useGlobalStore()
  const settings = rspc.createQuery(() => ({
    queryKey: ["settings.getSettings"]
  }))

  const settingsMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.setSettings"]
  }))

  // A hook set here runs for every instance, so a test needs one to run in.
  const instances = createMemo(() =>
    (globalStore.instances.data ?? []).filter(
      (instance) => instance.status.status === "valid"
    )
  )
  const [picked, setPicked] = createSignal<number>()
  const testInstance = () => {
    const chosen = picked()
    return chosen !== undefined &&
      instances().some((instance) => instance.id === chosen)
      ? chosen
      : defaultTestInstance(instances())
  }
  const nameOf = (id: string | undefined) =>
    instances().find((instance) => String(instance.id) === id)?.name ?? ""

  const TestInstance = () => (
    <Select
      value={testInstance() === undefined ? undefined : String(testInstance())}
      onChange={(value) => {
        if (value) setPicked(Number(value))
      }}
      options={instances().map((instance) => String(instance.id))}
      disabled={instances().length === 0}
      itemComponent={(props) => (
        <SelectItem item={props.item}>{nameOf(props.item.rawValue)}</SelectItem>
      )}
    >
      <SelectTrigger
        class="max-w-48"
        aria-label={t("settings:_trn_hook_test_instance")}
      >
        <SelectValue<string>>
          {(state) => (
            <span class="truncate">{nameOf(state.selectedOption())}</span>
          )}
        </SelectValue>
      </SelectTrigger>
      <SelectContent />
    </Select>
  )

  const [preLaunchDraft, setPreLaunchDraft] = createSignal<string>()
  const [postExitDraft, setPostExitDraft] = createSignal<string>()

  return (
    <>
      <PageTitle>
        <Trans key="settings:_trn_custom_commands_title" />
      </PageTitle>
      <RowsContainer>
        <Row forceContentBelow>
          <Title
            description={<Trans key="settings:_trn_pre_launch_hook_text" />}
          >
            <Trans key="settings:_trn_pre_launch_hook_title" />
          </Title>
          <HookTest
            instance={testInstance}
            kind="PreLaunch"
            command={() =>
              preLaunchDraft() ?? settings.data?.preLaunchHook ?? ""
            }
            beside={<TestInstance />}
          >
            <Input
              value={settings.data?.preLaunchHook || ""}
              onInput={(e) => setPreLaunchDraft(e.currentTarget.value)}
              onChange={(e) => {
                settingsMutation.mutate({
                  preLaunchHook: {
                    Set: e.currentTarget.value.trim() || null
                  }
                })
              }}
            />
          </HookTest>
        </Row>
        <Row forceContentBelow>
          <Title
            description={<Trans key="settings:_trn_post_exit_hook_text" />}
          >
            <Trans key="settings:_trn_post_exit_hook_title" />
          </Title>
          <HookTest
            instance={testInstance}
            kind="PostExit"
            command={() => postExitDraft() ?? settings.data?.postExitHook ?? ""}
            beside={<TestInstance />}
          >
            <Input
              value={settings.data?.postExitHook || ""}
              onInput={(e) => setPostExitDraft(e.currentTarget.value)}
              onChange={(e) => {
                settingsMutation.mutate({
                  postExitHook: {
                    Set: e.currentTarget.value.trim() || null
                  }
                })
              }}
            />
          </HookTest>
        </Row>
        <Row forceContentBelow>
          <Title
            description={<Trans key="settings:_trn_wrapper_command_text" />}
          >
            <Trans key="settings:_trn_wrapper_command_title" />
          </Title>
          <Input
            value={settings.data?.wrapperCommand || ""}
            onChange={(e) => {
              settingsMutation.mutate({
                wrapperCommand: {
                  Set: e.currentTarget.value.trim() || null
                }
              })
            }}
          />
        </Row>
      </RowsContainer>
    </>
  )
}

export default CustomCommands
