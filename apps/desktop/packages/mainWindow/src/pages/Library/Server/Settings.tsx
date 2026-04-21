import { rspc } from "@/utils/rspcClient"
import { Input, Slider, Switch } from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import { createEffect, createSignal, Show } from "solid-js"
import { FEServerDetails } from "@gd/core_module/bindings"
import { generateSequence } from "@/utils/helpers"
import Title from "@/pages/Settings/components/Title"
import Row from "@/pages/Settings/components/Row"
import RowsContainer from "@/pages/Settings/components/RowsContainer"
import RightHandSide from "@/pages/Settings/components/RightHandSide"

interface SettingsProps {
  serverDetails: FEServerDetails
  totalRam: number | undefined
}

const Settings = (props: SettingsProps) => {
  const [t] = useTransContext()
  const [name, setName] = createSignal("")
  const [xmx, setXmx] = createSignal(2048)
  const [xms, setXms] = createSignal(1024)
  const [extraJavaArgs, setExtraJavaArgs] = createSignal("")
  const [autoRestart, setAutoRestart] = createSignal(false)

  const updateServerMutation = rspc.createMutation(() => ({
    mutationKey: ["server.updateServer"]
  }))

  createEffect(() => {
    const d = props.serverDetails
    if (!d) return
    setName(d.name)
    setXmx(d.xmx)
    setXms(d.xms)
    setExtraJavaArgs(d.extraJavaArgs)
    setAutoRestart(d.autoRestart)
  })

  const save = (update: Record<string, unknown>) => {
    updateServerMutation.mutate({
      id: props.serverDetails.id,
      ...update
    })
  }

  const totalRamMb = () => {
    const ram = props.totalRam
    return ram ? Math.floor(ram / (1024 * 1024)) : 16384
  }

  return (
    <div>
      <h3 class="text-lightSlate-100 mb-0 mt-4 flex items-center gap-2 text-xl font-medium">
        <div class="i-hugeicons:settings-02 h-5 w-5" />
        <Trans key="instances:_trn_server_settings_general" />
      </h3>
      <RowsContainer>
        <Row>
          <Title
            description={
              t("instances:_trn_server_settings_name_description") || undefined
            }
          >
            <Trans key="instances:_trn_server_settings_name" />
          </Title>
          <RightHandSide>
            <Input
              class="w-60"
              value={name()}
              onInput={(e) => setName(e.target.value)}
              onBlur={() => {
                if (name() !== props.serverDetails.name) {
                  save({ name: name() })
                }
              }}
              onKeyDown={(e: KeyboardEvent) => {
                if (e.key === "Enter") {
                  if (name() !== props.serverDetails.name) {
                    save({ name: name() })
                  }
                }
              }}
            />
          </RightHandSide>
        </Row>

        <Row>
          <Title>
            <Trans key="instances:_trn_server_settings_auto_restart" />
          </Title>
          <RightHandSide>
            <Switch
              checked={autoRestart()}
              onChange={(val) => {
                setAutoRestart(val)
                save({ autoRestart: val })
              }}
            />
          </RightHandSide>
        </Row>
      </RowsContainer>

      <h3 class="text-lightSlate-100 mb-0 mt-8 flex items-center gap-2 text-xl font-medium">
        <div class="i-hugeicons:coffee-beans h-5 w-5" />
        <Trans key="instances:_trn_server_settings_java" />
      </h3>
      <RowsContainer>
        <Row>
          <Title
            description={
              xmx() >= 1024
                ? `${(xmx() / 1024).toFixed(1).replace(/\.0$/, "")} GB`
                : `${xmx()} MB`
            }
          >
            <Trans key="instances:_trn_server_settings_xmx" />
          </Title>
          <RightHandSide class="max-w-100 flex-1">
            <Slider
              min={512}
              max={Math.min(totalRamMb(), 32768)}
              steps={256}
              value={xmx()}
              marks={generateSequence(2048, Math.min(totalRamMb(), 32768))}
              tooltipFormat={(val) =>
                val >= 1024
                  ? `${(val / 1024).toFixed(1).replace(/\.0$/, "")} GB`
                  : `${val} MB`
              }
              onChange={(val) => {
                setXmx(val)
                if (xms() > val) setXms(val)
              }}
              OnRelease={(val) => {
                if (val !== props.serverDetails.xmx) {
                  save({
                    xmx: val,
                    ...(xms() > val ? { xms: val } : {})
                  })
                }
              }}
            />
          </RightHandSide>
        </Row>
        <Show when={xmx() > totalRamMb() * 0.8}>
          <div class="flex items-center gap-2 px-2 pb-4 text-sm text-yellow-500">
            <div class="i-hugeicons:alert-02 h-4 w-4 shrink-0" />
            <Trans key="java:_trn_ram_warning_high_allocation" />
          </div>
        </Show>

        <Row>
          <Title
            description={
              xms() >= 1024
                ? `${(xms() / 1024).toFixed(1).replace(/\.0$/, "")} GB`
                : `${xms()} MB`
            }
          >
            <Trans key="instances:_trn_server_settings_xms" />
          </Title>
          <RightHandSide class="max-w-100 flex-1">
            <Slider
              min={256}
              max={xmx()}
              steps={256}
              value={xms()}
              marks={generateSequence(1024, xmx())}
              tooltipFormat={(val) =>
                val >= 1024
                  ? `${(val / 1024).toFixed(1).replace(/\.0$/, "")} GB`
                  : `${val} MB`
              }
              onChange={(val) => {
                setXms(val)
              }}
              OnRelease={(val) => {
                if (val !== props.serverDetails.xms) {
                  save({ xms: val })
                }
              }}
            />
          </RightHandSide>
        </Row>

        <Row>
          <Title>
            <Trans key="instances:_trn_server_settings_extra_args" />
          </Title>
          <RightHandSide>
            <Input
              class="w-80"
              value={extraJavaArgs()}
              placeholder="-XX:+UseG1GC ..."
              onInput={(e) => setExtraJavaArgs(e.target.value)}
              onBlur={() => {
                if (extraJavaArgs() !== props.serverDetails.extraJavaArgs) {
                  save({ extraJavaArgs: extraJavaArgs() || null })
                }
              }}
              onKeyDown={(e: KeyboardEvent) => {
                if (e.key === "Enter") {
                  if (extraJavaArgs() !== props.serverDetails.extraJavaArgs) {
                    save({ extraJavaArgs: extraJavaArgs() || null })
                  }
                }
              }}
            />
          </RightHandSide>
        </Row>
      </RowsContainer>
    </div>
  )
}

export default Settings
