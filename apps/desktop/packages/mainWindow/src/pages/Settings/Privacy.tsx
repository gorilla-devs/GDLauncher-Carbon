import { Button } from "@gd/ui"
import { Trans } from "@gd/i18n"
import PageTitle from "./components/PageTitle"
import RowsContainer from "./components/RowsContainer"
import Row from "./components/Row"
import Title from "./components/Title"
import RightHandSide from "./components/RightHandSide"
import { useModal } from "@/managers/ModalsManager"

const Privacy = () => {
  const modalsContext = useModal()

  return (
    <>
      <PageTitle>
        <Trans key="settings:_trn_privacy" />
      </PageTitle>
      <RowsContainer>
        <Row>
          <Title
            description={<Trans key="settings:_trn_ads_personalization_text" />}
          >
            <Trans key="settings:_trn_ads_personalization_title" />
          </Title>
          <RightHandSide>
            <Button
              type="secondary"
              size="small"
              rounded={false}
              onClick={() => {
                window?.openCMPWindow()
              }}
            >
              <Trans key="auth:_trn_login.manage" />
            </Button>
          </RightHandSide>
        </Row>
        <Row forceContentBelow>
          <Title description={<Trans key="settings:_trn_documents_text" />}>
            <Trans key="settings:_trn_documents_title" />
          </Title>
          <div class="flex gap-4">
            <Button
              type="secondary"
              size="small"
              rounded={false}
              onClick={() => {
                modalsContext?.openModal({
                  name: "privacyStatement"
                })
              }}
            >
              <Trans key="settings:_trn_privacy_policy" />
            </Button>

            <Button
              type="secondary"
              size="small"
              rounded={false}
              onClick={() => {
                modalsContext?.openModal({
                  name: "termsAndConditions"
                })
              }}
            >
              <Trans key="settings:_trn_terms_of_service" />
            </Button>
          </div>
        </Row>
      </RowsContainer>
    </>
  )
}

export default Privacy
