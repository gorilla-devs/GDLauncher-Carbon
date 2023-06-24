import { Button } from "@gd/ui";
import { Trans } from "@gd/i18n";
import PageTitle from "./components/PageTitle";
import RowsContainer from "./components/RowsContainer";
import Row from "./components/Row";
import Title from "./components/Title";
import RightHandSide from "./components/RightHandSide";
import AdTrackingSettingsSlider from "@/components/AdTrackingSettingsSlider";
import { rspc } from "@/utils/rspcClient";

const Privacy = () => {
  const settings = rspc.createQuery(() => ["settings.getSettings"]);
  const settingsMutation = rspc.createMutation(["settings.setSettings"]);

  return (
    <>
      <PageTitle>
        <Trans
          key="settings.Privacy"
          options={{
            defaultValue: "Privacy",
          }}
        />
      </PageTitle>
      <RowsContainer>
        <Row>
          <Title
            description={<Trans key="settings.ads_personalization_text" />}
          >
            <Trans key="settings.ads_personalization_title" />
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
          <Title
            description={
              <div class="flex flex-col gap-4">
                <div>
                  <div class="text-yellow-400 mb-2 font-extrabold">
                    <Trans key="tracking.setting_disabled" />
                  </div>
                  <Trans key="tracking.setting_disabled_text" />
                </div>
                <div>
                  <div class="text-green-300 font-extrabold mb-2">
                    <Trans key="tracking.setting_anonymous" />
                  </div>
                  <Trans key="tracking.setting_anonymous_text" />
                </div>
                <div>
                  <div class="text-purple-500 font-extrabold mb-2">
                    <Trans key="tracking.setting_anonymous_with_session_recordings" />
                  </div>
                  <Trans key="tracking.setting_anonymous_with_session_recordings_text" />
                </div>
                <div>
                  <div class="text-fuchsia-400 font-extrabold mb-2">
                    <Trans key="tracking.settings_authenticated_with_session_recordings" />
                  </div>
                  <Trans key="tracking.settings_authenticated_with_session_recordings_text" />
                </div>
              </div>
            }
          >
            <Trans key="settings.metrics_level_title" />
          </Title>
          <RightHandSide class="w-280">
            <AdTrackingSettingsSlider
              metricLevel={settings.data?.metricsLevel}
              onChange={(metricsLevel) => {
                settingsMutation.mutate({ metricsLevel });
              }}
            />
          </RightHandSide>
        </Row>
      </RowsContainer>
    </>
  );
};

export default Privacy;
