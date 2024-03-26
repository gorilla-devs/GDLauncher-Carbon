import { Trans } from "@gd/i18n";
import { Input } from "@gd/ui";
import { rspc } from "@/utils/rspcClient";
import PageTitle from "./components/PageTitle";
import Row from "./components/Row";
import Title from "./components/Title";
import RowsContainer from "./components/RowsContainer";

const CustomCommands = () => {
  const settings = rspc.createQuery(() => ({
    queryKey: ["settings.getSettings"]
  }));

  const settingsMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.setSettings"]
  }));

  return (
    <>
      <PageTitle>
        <Trans key="settings:custom_commands_title" />
      </PageTitle>
      <RowsContainer>
        <Row forceContentBelow>
          <Title description={<Trans key="settings:pre_launch_hook_text" />}>
            <Trans key="settings:pre_launch_hook_title" />
          </Title>
          <Input
            value={settings.data?.preLaunchHook || ""}
            onChange={(e) => {
              settingsMutation.mutate({
                preLaunchHook: {
                  Set: e.currentTarget.value.trim() || null
                }
              });
            }}
          />
        </Row>
        <Row forceContentBelow>
          <Title description={<Trans key="settings:post_exit_hook_text" />}>
            <Trans key="settings:post_exit_hook_title" />
          </Title>
          <Input
            value={settings.data?.postExitHook || ""}
            onChange={(e) => {
              settingsMutation.mutate({
                postExitHook: {
                  Set: e.currentTarget.value.trim() || null
                }
              });
            }}
          />
        </Row>
        <Row forceContentBelow>
          <Title description={<Trans key="settings:wrapper_command_text" />}>
            <Trans key="settings:wrapper_command_title" />
          </Title>
          <Input
            value={settings.data?.wrapperCommand || ""}
            onChange={(e) => {
              settingsMutation.mutate({
                wrapperCommand: {
                  Set: e.currentTarget.value.trim() || null
                }
              });
            }}
          />
        </Row>
      </RowsContainer>
    </>
  );
};

export default CustomCommands;
