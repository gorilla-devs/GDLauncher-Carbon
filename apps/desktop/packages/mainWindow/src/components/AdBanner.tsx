/* eslint-disable i18next/no-literal-string */
import adSize from "@/utils/adhelper";
import { Show } from "solid-js";

export const AdsBanner = () => {
  let isSnapshotRelease = __APP_VERSION__.includes("-snapshot");
  return (
    <div
      style={{
        height: `${adSize.height}px`,
        width: `${adSize.width}px`,
      }}
    >
      <Show when={isSnapshotRelease}>
        <div class="flex flex-col w-full mb-8 box-border">
          <div class="flex w-full justify-center items-center h-10 bg-yellow-900 font-bold box-border">
            GDLauncher Snapshot Release
          </div>
          <div class="flex-wrap w-full p-4 border-1 border-lightSlate-600 border-x-solid border-b-solid box-border">
            This is a GDLauncher snapshot release.
            <br />
            This means that it is highly unstable and does not have auto
            updates.
          </div>
        </div>
      </Show>
      <owadview />
    </div>
  );
};
