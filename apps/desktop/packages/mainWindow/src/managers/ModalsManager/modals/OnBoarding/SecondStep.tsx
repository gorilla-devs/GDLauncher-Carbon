import RightHandSide from "@/pages/Settings/components/RightHandSide";
import Row from "@/pages/Settings/components/Row";
import RowsContainer from "@/pages/Settings/components/RowsContainer";
import Title from "@/pages/Settings/components/Title";
import { rspc } from "@/utils/rspcClient";
import { Trans } from "@gd/i18n";
import { Button, Switch } from "@gd/ui";

type Props = {
  nextStep: () => void;
  prevStep: () => void;
};

const SecondStep = (props: Props) => {
  let settingsMutation = rspc.createMutation(["settings.setSettings"]);
  let settings = rspc.createQuery(() => ["settings.getSettings"]);
  return (
    <div class="flex flex-col justify-between h-full lg:w-160 box-border">
      <RowsContainer>
        <Row>
          <Title
            description={
              <Trans key="java.auto_manage_java_system_profiles_text" />
            }
          >
            <Trans key="java.auto_manage_java_system_profiles" />
          </Title>
          <RightHandSide>
            <Switch
              checked={settings.data?.autoManageJavaSystemProfiles}
              onChange={(e) => {
                settingsMutation.mutate({
                  autoManageJavaSystemProfiles: {
                    Set: e.target.checked
                  }
                });
              }}
            />
          </RightHandSide>
        </Row>
        <Row>
          <Title description={<Trans key="settings:show_news_text" />}>
            <Trans key="settings:show_news_title" />
          </Title>
          <RightHandSide>
            <Switch
              checked={settings.data?.showNews}
              onChange={(e) => {
                settingsMutation.mutate({
                  showNews: {
                    Set: e.currentTarget.checked
                  }
                });
              }}
            />
          </RightHandSide>
        </Row>
        <Row>
          <Title
            description={
              <Trans key="settings:deletion_through_recycle_bin_text" />
            }
          >
            <Trans key="settings:deletion_through_recycle_bin_title" />
          </Title>
          <RightHandSide>
            <Switch
              checked={settings.data?.deletionThroughRecycleBin}
              onChange={(e) => {
                settingsMutation.mutate({
                  deletionThroughRecycleBin: {
                    Set: e.currentTarget.checked
                  }
                });
              }}
            />
          </RightHandSide>
        </Row>
        <Row>
          <Trans key="onboarding.manage_more_options_from_settings" />
        </Row>
      </RowsContainer>
      <div class="flex justify-between w-full">
        <Button
          type="secondary"
          size="large"
          onClick={() => {
            props.prevStep();
          }}
        >
          <Trans key="onboarding.prev" />
        </Button>
        <Button
          onClick={() => {
            props.nextStep();
          }}
          size="large"
        >
          <Trans key="onboarding.next" />
        </Button>
      </div>
    </div>
  );
};

export default SecondStep;
