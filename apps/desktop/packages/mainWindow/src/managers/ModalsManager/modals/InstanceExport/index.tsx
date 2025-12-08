import { ModalProps } from "../.."
import ModalLayout from "../../ModalLayout"
import ExportFormat from "./atoms/ExportFormat"
import ExportVersion from "./atoms/ExportVersion"
import FilesSelection from "./atoms/FilesSelection"
import SelfContainedArchive from "./atoms/SelfContainedArchive"
import ExportPath from "./atoms/ExportPath"
import BeginExport from "./atoms/BeginExport"
import { Match, Switch, createSignal } from "solid-js"
import { ExportTarget } from "@gd/core_module/bindings"
import { createStore } from "solid-js/store"
import Exporting from "./atoms/Exporting"
import ExportDone from "./atoms/ExportDone"

const [exportStep, setExportStep] = createSignal(0)
export { exportStep, setExportStep }

interface Props {
  instanceId: number
}

interface IPayload {
  instance_id: number | undefined
  target: ExportTarget
  save_path: string | undefined
  self_contained_addons_bundling: boolean
  filter: {}
  version: string
}

const [payload, setPayload] = createStore<IPayload>({
  instance_id: undefined,
  target: "Curseforge",
  save_path: undefined,
  self_contained_addons_bundling: false,
  filter: { entries: {} },
  version: "1.0.0"
})

export { payload, setPayload }

const InstanceExport = (props: ModalProps) => {
  const data: () => Props = () => props.data
  const instanceId = () => data()?.instanceId
  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      noPadding={true}
    >
      <div class="w-120 flex flex-col p-4">
        <Switch>
          <Match when={exportStep() === 0}>
            <ExportFormat />
            <ExportVersion />
            <FilesSelection instanceId={instanceId()} />
            <SelfContainedArchive />
            <ExportPath />
            <BeginExport instanceId={instanceId()} />
          </Match>
          <Match when={exportStep() === 1}>
            <Exporting />
          </Match>
          <Match when={exportStep() === 2}>
            <ExportDone path={payload.save_path!} />
          </Match>
        </Switch>
      </div>
    </ModalLayout>
  )
}

export default InstanceExport
