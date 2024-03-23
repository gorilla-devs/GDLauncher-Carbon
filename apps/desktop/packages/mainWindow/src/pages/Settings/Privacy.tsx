import { Button, Switch } from "@gd/ui";
import { Trans } from "@gd/i18n";
import PageTitle from "./components/PageTitle";
import RowsContainer from "./components/RowsContainer";
import Row from "./components/Row";
import Title from "./components/Title";
import RightHandSide from "./components/RightHandSide";
import { rspc } from "@/utils/rspcClient";

const Privacy = () => {
  const settings = rspc.createQuery(() => ({
    queryKey: ["settings.getSettings"]
  }));

  const settingsMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.setSettings"]
  }));

  return (
    <>
      <PageTitle>
        <Trans key="settings:Privacy" />
      </PageTitle>
      <RowsContainer>
        <Row>
          <Title
            description={<Trans key="settings:ads_personalization_text" />}
          >
            <Trans key="settings:ads_personalization_title" />
          </Title>
          <RightHandSide>
            <Button
              type="secondary"
              size="small"
              rounded={false}
              onClick={() => {
                window?.openCMPWindow();
              }}
            >
              <Trans key="login.manage" />
            </Button>
          </RightHandSide>
        </Row>
        <Row>
          <Title description={<Trans key="settings:enable_metrics_text" />}>
            <Trans key="settings:enable_metrics_title" />
          </Title>
          <RightHandSide>
            <Switch
              checked={settings.data?.metricsEnabled}
              onChange={(e) => {
                settingsMutation.mutate({
                  metricsEnabled: {
                    Set: e.currentTarget.checked
                  }
                });
              }}
            />
          </RightHandSide>
        </Row>
      </RowsContainer>
    </>
  );
};

export default Privacy;
