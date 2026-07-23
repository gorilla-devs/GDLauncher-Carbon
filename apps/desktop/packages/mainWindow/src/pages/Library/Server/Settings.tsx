import { rspc } from "@/utils/rspcClient"
import { Button, Input, Slider, Switch } from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import { createEffect, createSignal, on, Show } from "solid-js"
import { FEServerDetails } from "@gd/core_module/bindings"
import { generateSequence } from "@/utils/helpers"
import Title from "@/pages/Settings/components/Title"
import Row from "@/pages/Settings/components/Row"
import RowsContainer from "@/pages/Settings/components/RowsContainer"
import RightHandSide from "@/pages/Settings/components/RightHandSide"
import { useModal } from "@/managers/ModalsManager"

interface SettingsProps {
  serverDetails: FEServerDetails
  totalRam: number | undefined
}

const Settings = (props: SettingsProps) => {
  const [t] = useTransContext()
  const modalsContext = useModal()
  const [name, setName] = createSignal("")
  const [xmx, setXmx] = createSignal(2048)
  const [xms, setXms] = createSignal(1024)
  const [extraJavaArgs, setExtraJavaArgs] = createSignal("")
  const [autoRestart, setAutoRestart] = createSignal(false)

  const updateServerMutation = rspc.createMutation(() => ({
    mutationKey: ["server.updateServer"]
  }))

  // Reset the local form signals only when the viewed server changes, not on
  // every `serverDetails` refetch (e.g. the instant-save invalidation from
  // `save()` below) — otherwise a refetch mid-edit clobbers whatever the
  // user is still typing in a sibling field.
  createEffect(
    on(
      () => props.serverDetails?.id,
      () => {
        const d = props.serverDetails
        if (!d) return
        setName(d.name)
        setXmx(d.xmx)
        setXms(d.xms)
        setExtraJavaArgs(d.extraJavaArgs)
        setAutoRestart(d.autoRestart)
      }
    )
  )

  const save = (
    update: Partial<{
      name: string | null
      xmx: number | null
      xms: number | null
      extraJavaArgs: string | null
      autoRestart: boolean | null
    }>
  ) => {
    updateServerMutation.mutate({
      id: props.serverDetails.id,
      name: update.name ?? null,
      xmx: update.xmx ?? null,
      xms: update.xms ?? null,
      // extraJavaArgs is a double-Option on the backend (`null` means
      // "clear", distinct from "leave untouched"), unlike the single-Option
      // fields above where `null` already means "leave untouched". Re-send
      // the current value when this call isn't the one changing it, so
      // saving e.g. xmx alone can't silently blank the java args.
      extraJavaArgs:
        update.extraJavaArgs !== undefined
          ? update.extraJavaArgs
          : props.serverDetails.extraJavaArgs,
      autoRestart: update.autoRestart ?? null
    })
  }

  const totalRamMb = () => {
    const ram = props.totalRam
    return ram ? Math.floor(ram / (1024 * 1024)) : 16384
  }

  return (
    <div>
      <h3 class="text-lightSlate-100 mb-0 mt-4 flex items-center gap-2 text-xl font-medium">
        <div class="i-hugeicons:package h-5 w-5" />
        <Trans key="instances:_trn_instance_settings.modpack_info" />
      </h3>
      <RowsContainer>
        <Row>
          <Title>
            <Trans key="instances:_trn_instance_settings.reinstall" />
          </Title>
          <RightHandSide>
            <Button
              type="secondary"
              // Backend's reinstall_server_from_modpack only accepts a
              // server in the Stopped state — disable here too so it's not
              // a click-and-error round trip.
              disabled={
                !props.serverDetails.modpackInfo ||
                props.serverDetails.state.status !== "stopped"
              }
              onClick={() => {
                modalsContext?.openModal(
                  {
                    name: "confirmReinstall"
                  },
                  {
                    id: props.serverDetails.id,
                    name: props.serverDetails.name,
                    isServer: true
                  }
                )
              }}
            >
              <i class="i-hugeicons:refresh h-5 w-5" />
              <Trans key="instances:_trn_instance_settings.reinstall" />
            </Button>
          </RightHandSide>
        </Row>
      </RowsContainer>
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
              onChange={(e) => {
                const val = e.currentTarget.checked
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
          <Title description={<Trans key="java:_trn_java_arguments_hint" />}>
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
