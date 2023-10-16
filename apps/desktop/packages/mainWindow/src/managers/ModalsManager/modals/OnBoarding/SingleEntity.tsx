import { rspc } from "@/utils/rspcClient";
import { ImportEntityStatus } from "@gd/core_module/bindings";
import { Input } from "@gd/ui";
import { Setter, createEffect, createResource, createSignal } from "solid-js";

const SingleEntity = (props: {
  entity: ImportEntityStatus;
  setEntity: Setter<ImportEntityStatus | undefined>;
}) => {
  const [runtimePath, setRuntimePath] = createSignal<string | undefined>(
    undefined
  );
  const [initialRuntimePath] = createResource(() => {
    return window.getInitialRuntimePath();
  });

  const [currentRuntimePath] = createResource(() => {
    return window.getRuntimePath();
  });
  const entityDefaultPath = rspc.createQuery(() => [
    "instance.getImportEntityDefaultPath",
    props.entity.entity
  ]);

  createEffect(() => {
    if (!entityDefaultPath.data) {
      if (currentRuntimePath() === undefined) {
        return;
      } else {
        setRuntimePath(currentRuntimePath()!);
        return;
      }
    }

    setRuntimePath(entityDefaultPath.data!);
  });

  return (
    <>
      <div class="w-full flex justify-between items-center pt-6">
        <div
          onClick={() => props.setEntity(undefined)}
          class="i-mingcute:arrow-left-fill text-zinc-400 text-xl font-bold cursor-pointer"
        ></div>
        <span class="font-bold">{props.entity.entity}</span>
        <div></div>
      </div>
      <div class="border-red-400 border-2 border-solid flex-1 w-full flex flex-col items-center justify-center">
        <div class="flex items-center justify-between w-full gap-2">
          <span>Scan target path:</span>
          <Input
            value={runtimePath()}
            class="flex-1 border-2 border-solid border-zinc-500"
            icon={
              <div class="i-ic:round-folder text-2xl text-yellow-300 cursor-pointer"></div>
            }
          />
        </div>
      </div>
    </>
  );
};
export default SingleEntity;

// GET_IMPORT_ENTITY_DEFAULT_PATH => returns an Option<String> of the default search path for the given import type
// SET_IMPORT_SCAN_TARGET => begins scanning at the given (path, import type). if GET_IMPORT_ENTITY_DEFAULT_PATH returns some you can call this immediately
// GET_IMPORT_SCAN_STATUS => gets the status of the current scan. the status includes scanning (if the scanner is currently active) and status, the current status of the scanner. the status can be NoResults, SingleResult(result), or MultiResult(Vec<result>). result is an ImportEntity which may either be valid and contain the filename and instance name, or invalid and contain the filename and a translation for the invalid reason. GET_IMPORT_SCAN_STATUS will fail if there is not currently an active scan.
// CANCEL_IMPORT_SCAN => stops any running scans, call this if the modal closes
// IMPORT_INSTANCE => starts importing an instance, returns the taskid of the prepare task
