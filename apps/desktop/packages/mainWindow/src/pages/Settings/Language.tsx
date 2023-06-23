/* eslint-disable i18next/no-literal-string */
import { Trans, supportedLanguages, useTransContext } from "@gd/i18n";
import PageTitle from "./components/PageTitle";
import Row from "./components/Row";
import RowsContainer from "./components/RowsContainer";
import { Radio } from "@gd/ui";
import { For, createEffect } from "solid-js";
import { rspc, queryClient } from "@/utils/rspcClient";
import { FESettings } from "@gd/core_module/bindings";
import { createStore } from "solid-js/store";

const Language = () => {
  const [t, { getI18next }] = useTransContext();

  console.log(getI18next().getResourceBundle("english", "languages"));

  let data = rspc.createQuery(() => ["settings.getSettings"]);

  const [settings, setSettings] = createStore<FESettings | {}>({});

  const settingsMutation = rspc.createMutation(["settings.setSettings"], {});

  createEffect(() => {
    if (data.data) setSettings(data.data);
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
        <Row>
          <div class="w-full flex flex-col mt-6 divide-y divide-darkSlate-600">
            <Radio.group
              onChange={(value) => {
                settingsMutation.mutate({
                  language: value as string,
                });
              }}
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
