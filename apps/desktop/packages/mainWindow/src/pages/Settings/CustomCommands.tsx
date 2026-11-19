import { createMemo, createSignal } from "solid-js"
import { Trans } from "@gd/i18n"
import { Input } from "@gd/ui"
import { rspc } from "@/utils/rspcClient"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import HookTest from "@/components/HookTest"
import { pickable } from "@/components/InstancePicker"
import PageTitle from "./components/PageTitle"
import Row from "./components/Row"
import Title from "./components/Title"
import RowsContainer from "./components/RowsContainer"

/** The instances a launcher-wide hook can be tested in, in the order Test
 * offers them: the ones played, latest first, then the rest as they come. */
export function testOrder<T extends { last_played: string | null }>(
  instances: readonly T[]
): T[] {
  const played = instances
    .filter((instance) => instance.last_played !== null)
    .sort((a, b) => (a.last_played! < b.last_played! ? 1 : -1))
  return [
    ...played,
    ...instances.filter((instance) => instance.last_played === null)
  ]
}

const CustomCommands = () => {
  const globalStore = useGlobalStore()
  const settings = rspc.createQuery(() => ({
    queryKey: ["settings.getSettings"]
  }))

  const settingsMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.setSettings"]
  }))

  // A hook set here runs for every instance, so a test needs one to run in.
  const testIn = createMemo(() =>
    testOrder(
      (globalStore.instances.data ?? []).filter(
        (instance) => instance.status.status === "valid"
      )
    ).map(pickable)
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
            choices={testIn}
            kind="PreLaunch"
            command={() =>
              preLaunchDraft() ?? settings.data?.preLaunchHook ?? ""
            }
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
            choices={testIn}
            kind="PostExit"
            command={() => postExitDraft() ?? settings.data?.postExitHook ?? ""}
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
