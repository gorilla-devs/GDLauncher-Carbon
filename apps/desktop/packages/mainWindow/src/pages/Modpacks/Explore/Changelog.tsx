/* eslint-disable solid/no-innerhtml */
import { useParams, useRouteData } from "@solidjs/router";
import { Match, Show, Switch, createEffect, createSignal } from "solid-js";
import { Dropdown, Skeleton } from "@gd/ui";
import { rspc } from "@/utils/rspcClient";
import fetchData from "../modpack.overview";
import { FEFileIndex, FEModResponse } from "@gd/core_module/bindings";

const sortArrayByGameVersion = (arr: FEFileIndex[]) => {
  let sortedArr = [...arr];

  sortedArr.sort((a, b) => {
    let aVersion = a.gameVersion.split(".").map(Number);
    let bVersion = b.gameVersion.split(".").map(Number);

    for (let i = 0; i < aVersion.length; i++) {
      if (aVersion[i] > bVersion[i]) {
        return -1;
      }
      if (aVersion[i] < bVersion[i]) {
        return 1;
      }
    }

    return 0;
  });

  return sortedArr;
};

const Changelog = () => {
  const params = useParams();

  const routeData: ReturnType<typeof fetchData> = useRouteData();
  const lastFile = () =>
    routeData.modpackDetails?.data?.data.latestFiles[
      routeData.modpackDetails?.data?.data.latestFiles.length - 1
    ];
  const [fileId, setFileId] = createSignal<number | undefined>(undefined);
  const [changeLog, setChangelog] = createSignal<string | undefined>(undefined);

  createEffect(() => {
    const modpackId = parseInt(params.id, 10);
    if (fileId() !== undefined || lastFile()?.id !== undefined) {
      // eslint-disable-next-line solid/reactivity
      const changelogQuery = rspc.createQuery(() => [
        "modplatforms.curseforge.getModFileChangelog",
        {
          modId: modpackId,
          fileId: (fileId() as number) || (lastFile()?.id as number),
        },
      ]);
      setChangelog(changelogQuery.data?.data);
    }
  });

  const sortedFilesByMcVersion = () =>
    sortArrayByGameVersion(
      (routeData.modpackDetails.data as FEModResponse).data.latestFilesIndexes
    );

  return (
    <div>
      <Show when={routeData.modpackDetails.data}>
        <Dropdown
          options={sortedFilesByMcVersion().map((file) => ({
            key: file.fileId,
            label: file.filename,
          }))}
          value={fileId() || lastFile()?.id}
          onChange={(fileId) => {
            setFileId(fileId.key as number);
          }}
        />
      </Show>
      <Switch fallback={<Skeleton.modpackChangelogPage />}>
        <Match when={changeLog()}>
          <div innerHTML={changeLog()} />
        </Match>
      </Switch>
    </div>
  );
};

export default Changelog;
