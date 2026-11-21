import { Trans, useTransContext } from "@gd/i18n"
import { Collapsable, Switch } from "@gd/ui"
import { Show } from "solid-js"
import { rspc } from "@/utils/rspcClient"
import type { themeCustomization } from "@/themes/installed"
import { createPreferencesMutation } from "@/themes/preferences"
import Row from "../components/Row"
import Title from "../components/Title"
import RightHandSide from "../components/RightHandSide"

interface Props {
  /** What the saved theme has to customize. */
  customization: ReturnType<typeof themeCustomization>
}

/** The saved theme's own choices, behind a disclosure that starts closed. */
const CustomizeTheme = (props: Props) => {
  const [t] = useTransContext()
  const preferences = rspc.createQuery(() => ({
    queryKey: ["themes.getPreferences"]
  }))
  const setPreferences = createPreferencesMutation()

  return (
    <div data-testid="theme-customize">
      <Collapsable
        title={t("settings:_trn_theme_customize")}
        defaultOpened={false}
        noPadding
        size="small"
      >
        <Show when={props.customization.sounds}>
          <Row>
            <Title>
              <Trans key="settings:_trn_theme_sounds" />
            </Title>
            <RightHandSide>
              <Switch
                checked={preferences.data?.sounds ?? false}
                disabled={!preferences.data}
                onChange={(e) => {
                  setPreferences.mutate({ sounds: e.currentTarget.checked })
                }}
              />
            </RightHandSide>
          </Row>
        </Show>
        <Show when={props.customization.effects}>
          <Row>
            <Title>
              <Trans key="settings:_trn_theme_effects" />
            </Title>
            <RightHandSide>
              <Switch
                checked={preferences.data?.effects ?? false}
                disabled={!preferences.data}
                onChange={(e) => {
                  setPreferences.mutate({ effects: e.currentTarget.checked })
                }}
              />
            </RightHandSide>
          </Row>
        </Show>
      </Collapsable>
    </div>
  )
}

export default CustomizeTheme
