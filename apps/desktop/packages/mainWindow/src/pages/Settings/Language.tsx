import { Trans, supportedLanguages } from "@gd/i18n"
import { getLanguageKey, getLanguageNativeKey } from "@gd/i18n/helpers"
import PageTitle from "./components/PageTitle"
import Row from "./components/Row"
import RowsContainer from "./components/RowsContainer"
import { Radio } from "@gd/ui"
import { getOwner, runWithOwner } from "solid-js"
import { rspc } from "@/utils/rspcClient"
import Title from "./components/Title"
import changeLanguage from "@/utils/language"

const Language = () => {
  const settings = rspc.createQuery(() => ({
    queryKey: ["settings.getSettings"]
  }))

  const owner = getOwner()

  return (
    <>
      <PageTitle>
        <Trans key="settings:_trn_language" />
      </PageTitle>
      <RowsContainer>
        <Row class="flex-col justify-start">
          <Title class="w-full">
            <Trans key="settings:_trn_select_a_language" />
          </Title>
          <div class="flex w-full flex-col gap-1">
            <Radio.group
              onChange={(value) => {
                runWithOwner(owner, () => {
                  changeLanguage(value as string)
                })
              }}
              value={settings.data?.language}
              options={Object.entries(supportedLanguages).map(
                ([key, flagCode]) => {
                  const langCode = key as keyof typeof supportedLanguages
                  return {
                    value: key,
                    label: (
                      <div class="flex flex-1 items-center justify-between">
                        <span class="text-lightSlate-100 font-medium">
                          <Trans key={getLanguageNativeKey(langCode)} />
                        </span>
                        <div class="flex items-center gap-3">
                          <span class="text-lightSlate-400">
                            <Trans key={getLanguageKey(langCode)} />
                          </span>
                          <div
                            class={`i-emojione-v1:flag-for-${flagCode} h-5 w-5 shrink-0`}
                          />
                        </div>
                      </div>
                    )
                  }
                }
              )}
            />
          </div>
        </Row>
      </RowsContainer>
    </>
  )
}

export default Language
