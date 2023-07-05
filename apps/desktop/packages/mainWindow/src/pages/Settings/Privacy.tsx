import { Button } from "@gd/ui";
import { Trans } from "@gd/i18n";
import PageTitle from "./components/PageTitle";
import RowsContainer from "./components/RowsContainer";
import Row from "./components/Row";
import Title from "./components/Title";
import RightHandSide from "./components/RightHandSide";
import AdTrackingSettingsSlider from "@/components/AdTrackingSettingsSlider";
import { rspc } from "@/utils/rspcClient";
import { Show } from "solid-js";

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
                  <div class="flex gap-4">
                    <div
                      class="text-yellow-400 mb-2"
                      classList={{
                        "font-extrabold": settings.data?.metricsLevel === 0,
                      }}
                    >
                      <Trans key="tracking.setting_disabled" />
                    </div>
                    <Show when={settings.data?.metricsLevel === 0}>
                      <div class="w-6 h-4 text-white i-ri:arrow-left-fill" />
                    </Show>
                  </div>
                  <p
                    class="text-xs p-0"
                    classList={{
                      "text-white": settings.data?.metricsLevel === 0,
                    }}
                  >
                    <Trans key="tracking.setting_disabled_text" />
                  </p>
                </div>
                <div>
                  <div class="flex gap-4">
                    <div
                      class="text-green-300 mb-2"
                      classList={{
                        "font-extrabold": settings.data?.metricsLevel === 1,
                      }}
                    >
                      <Trans key="tracking.setting_anonymous" />
                    </div>
                    <Show when={settings.data?.metricsLevel === 1}>
                      <div class="w-6 h-4 text-white i-ri:arrow-left-fill" />
                    </Show>
                  </div>
                  <p
                    class="text-xs p-0"
                    classList={{
                      "text-white": settings.data?.metricsLevel === 1,
                    }}
                  >
                    <Trans key="tracking.setting_anonymous_text" />
                  </p>
                </div>
                <div>
                  <div class="flex gap-4">
                    <div
                      class="text-purple-500 mb-2"
                      classList={{
                        "font-extrabold": settings.data?.metricsLevel === 2,
                      }}
                    >
                      <Trans key="tracking.setting_anonymous_with_session_recordings" />
                    </div>
                    <Show when={settings.data?.metricsLevel === 2}>
                      <div class="w-6 h-4 text-white i-ri:arrow-left-fill" />
                    </Show>
                  </div>
                  <p
                    class="text-xs p-0"
                    classList={{
                      "text-white": settings.data?.metricsLevel === 2,
                    }}
                  >
                    <Trans key="tracking.setting_anonymous_with_session_recordings_text" />
                  </p>
                </div>
                <div>
                  <div class="flex gap-4">
                    <div
                      class="text-fuchsia-400 mb-2"
                      classList={{
                        "font-extrabold": settings.data?.metricsLevel === 3,
                      }}
                    >
                      <Trans key="tracking.settings_authenticated_with_session_recordings" />
                    </div>
                    <Show when={settings.data?.metricsLevel === 3}>
                      <div class="w-6 h-4 text-white i-ri:arrow-left-fill" />
                    </Show>
                  </div>
                  <p
                    class="text-xs p-0"
                    classList={{
                      "text-white": settings.data?.metricsLevel === 3,
                    }}
                  >
                    <Trans key="tracking.settings_authenticated_with_session_recordings_text" />
                  </p>
                </div>
              </div>
            }
          >
            <Trans key="settings.metrics_level_title" />
          </Title>
          <RightHandSide class="w-250">
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
