/* eslint-disable i18next/no-literal-string */
import { Trans, supportedLanguages } from "@gd/i18n";
import PageTitle from "./components/PageTitle";
import Row from "./components/Row";
import RowsContainer from "./components/RowsContainer";
import { Radio } from "@gd/ui";
import { For } from "solid-js";
import { rspc, queryClient } from "@/utils/rspcClient";
import Title from "./components/Title";

const Language = () => {
  let settings = rspc.createQuery(() => ["settings.getSettings"]);

  // @ts-ignore

  const settingsMutation = rspc.createMutation(["settings.setSettings"], {
    onMutate: (newSettings) => {
      queryClient.setQueryData(["settings.getSettings"], newSettings);
    },
  });

  return (
    <>
      <PageTitle>
        <Trans
          key="settings.language"
          options={{
            defaultValue: "Language",
          }}
        />
      </PageTitle>
      <RowsContainer>
        <Row class="flex-col justify-start">
          <Title class="w-full">
            <Trans key="settings.select_a_language" />
          </Title>
          <div class="w-full flex flex-col mt-6 divide-y divide-darkSlate-600">
            <Radio.group
              onChange={(value) => {
                settingsMutation.mutate({
                  language: value as string,
                });
              }}
              value={settings.data?.language}
            >
              <For each={supportedLanguages}>
                {(item) => (
                  <div class="h-12 flex items-center">
                    <Radio value={item}>
                      <div class="w-full flex justify-between">
                        <div>
                          <Trans key={`languages:${item}_native`} />
                        </div>
                      </div>
                    </Radio>
                  </div>
                )}
              </For>
            </Radio.group>
          </div>
        </Row>
      </RowsContainer>
    </>
  );
};

export default Language;
