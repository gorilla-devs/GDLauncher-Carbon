import { Trans, useTransContext } from "@gd/i18n"
import { Collapsable, Switch } from "@gd/ui"
import { rspc } from "@/utils/rspcClient"
import { createPreferencesMutation } from "@/themes/preferences"
import Row from "../components/Row"
import Title from "../components/Title"
import RightHandSide from "../components/RightHandSide"

/** The theme preferences, behind a disclosure that starts closed. Shown with
 *  every theme: they're global, and apply to any theme with sounds or
 *  effects, whichever is chosen later. */
const CustomizeTheme = () => {
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
      </Collapsable>
    </div>
  )
}

export default CustomizeTheme
