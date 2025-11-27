import RightHandSide from "@/pages/Settings/components/RightHandSide"
import Row from "@/pages/Settings/components/Row"
import RowsContainer from "@/pages/Settings/components/RowsContainer"
import Title from "@/pages/Settings/components/Title"
import { rspc } from "@/utils/rspcClient"
import { Trans } from "@gd/i18n"
import { Button, Switch } from "@gd/ui"

interface Props {
  nextStep: () => void
  prevStep: () => void
}

const SecondStep = (props: Props) => {
  const settingsMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.setSettings"]
  }))
  const settings = rspc.createQuery(() => ({
    queryKey: ["settings.getSettings"]
  }))

  return (
    <div class="lg:w-160 box-border flex h-full flex-col justify-between">
      <RowsContainer>
        <Row>
          <Title
            description={
              <Trans key="java:_trn_auto_manage_java_system_profiles_text" />
            }
          >
            <Trans key="java:_trn_auto_manage_java_system_profiles" />
          </Title>
          <RightHandSide>
            <Switch
              checked={settings.data?.autoManageJavaSystemProfiles}
              onChange={(e) => {
                settingsMutation.mutate({
                  autoManageJavaSystemProfiles: {
                    Set: e.target.checked
                  }
                })
              }}
            />
          </RightHandSide>
        </Row>
        <Row>
          <Title
            description={
              <Trans key="settings:_trn_deletion_through_recycle_bin_text" />
            }
          >
            <Trans key="settings:_trn_deletion_through_recycle_bin_title" />
          </Title>
          <RightHandSide>
            <Switch
              checked={settings.data?.deletionThroughRecycleBin}
              onChange={(e) => {
                settingsMutation.mutate({
                  deletionThroughRecycleBin: {
                    Set: e.currentTarget.checked
                  }
                })
              }}
            />
          </RightHandSide>
        </Row>
        <Row>
          <Trans key="onboarding:_trn_manage_more_options_from_settings" />
        </Row>
      </RowsContainer>
      <div class="flex w-full justify-between">
        <Button
          type="secondary"
          size="large"
          onClick={() => {
            props.prevStep()
          }}
        >
          <Trans key="onboarding:_trn_prev" />
        </Button>
        <Button
          onClick={() => {
            props.nextStep()
          }}
          size="large"
        >
          <Trans key="onboarding:_trn_next" />
        </Button>
      </div>
    </div>
  )
}

export default SecondStep
