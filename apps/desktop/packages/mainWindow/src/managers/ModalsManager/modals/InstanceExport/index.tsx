import { ModalProps } from "../..";
import ModalLayout from "../../ModalLayout";
import ExportFormat from "./atoms/ExportFormat";
import FilesSelection from "./atoms/FilesSelection";
import FilesBundle from "./atoms/FilesBundle";
import ExportPath from "./atoms/ExportPath";
import BeginExport from "./atoms/BeginExport";
import { Match, Switch, createSignal } from "solid-js";
import { ExportTarget } from "@gd/core_module/bindings";
import { createStore } from "solid-js/store";
import Exporting from "./atoms/Exporting";
import ExportDone from "./atoms/ExportDone";

const [exportStep, setExportStep] = createSignal(0);
export { exportStep, setExportStep };
interface IPayload {
  instance_id: number | undefined;
  target: ExportTarget;
  save_path: string | undefined;
  link_mods: boolean;
  filter: {};
}

const [payload, setPayload] = createStore<IPayload>({
  instance_id: undefined,
  target: "Curseforge",
  save_path: undefined,
  link_mods: true,
  filter: { entries: {} }
});
export { payload, setPayload };
const InstanceExport = (props: ModalProps) => {
  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      overflowHiddenDisabled={true}
      noPadding={true}
      scrollable="overflow-y-scroll scrollbar-hide"
      // height="h-96"
    >
      <div class="flex flex-col p-4 w-120">
        <Switch>
          <Match when={exportStep() === 0}>
            <ExportFormat />
            {/* <ExportNameVersion /> */}
            <FilesSelection />
            <FilesBundle />
            <ExportPath />
            <BeginExport />
          </Match>
          <Match when={exportStep() === 1}>
            <Exporting />
          </Match>
          <Match when={exportStep() === 2}>
            <ExportDone path={payload.save_path as string} />
          </Match>
        </Switch>
      </div>
    </ModalLayout>
  );
};

export default InstanceExport;
