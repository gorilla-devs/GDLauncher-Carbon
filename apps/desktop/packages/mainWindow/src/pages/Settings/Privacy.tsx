/* eslint-disable i18next/no-literal-string */
import { Button } from "@gd/ui";
import { Trans } from "@gd/i18n";
import PageTitle from "./components/PageTitle";
import RowsContainer from "./components/RowsContainer";
import Row from "./components/Row";
import Title from "./components/Title";
import RightHandSide from "./components/RightHandSide";

const Privacy = () => {
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
          <Title description={<Trans key="settings.metrics_level_text" />}>
            <Trans key="settings.metrics_level_title" />
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
      </RowsContainer>
    </>
  );
};

export default Privacy;
