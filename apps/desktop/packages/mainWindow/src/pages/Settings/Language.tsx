/* eslint-disable i18next/no-literal-string */
import { Trans, supportedLanguages } from "@gd/i18n";
import PageTitle from "./components/PageTitle";
import Row from "./components/Row";
import RowsContainer from "./components/RowsContainer";
import { Radio } from "@gd/ui";
import { getOwner, runWithOwner } from "solid-js";
import { rspc } from "@/utils/rspcClient";
import Title from "./components/Title";
import changeLanguage from "@/utils/language";

const Language = () => {
  let settings = rspc.createQuery(() => ({
    queryKey: ["settings.getSettings"]
  }));

  const owner = getOwner();

  return (
    <>
      <PageTitle>
        <Trans key="settings:language" />
      </PageTitle>
      <RowsContainer>
        <Row class="flex-col justify-start">
          <Title class="w-full">
            <Trans key="settings:select_a_language" />
          </Title>
          <div class="w-full flex flex-col divide-y divide-darkSlate-600">
            <Radio.group
              onChange={(value) => {
                runWithOwner(owner, () => {
                  changeLanguage(value as string);
                });
              }}
              value={settings.data?.language}
              options={Object.entries(supportedLanguages).map(
                ([key, value]) => ({
                  value: key,
                  label: (
                    <div class="w-full flex justify-between">
                      <div class="flex items-center gap-2">
                        <div
                          class={`h-5 w-5 i-emojione-v1:flag-for-${value}`}
                        />
                        <Trans key={`languages:${key}_native`} />
                      </div>
                    </div>
                  )
                })
              )}
            />
          </div>
        </Row>
      </RowsContainer>
    </>
  );
};

export default Language;
