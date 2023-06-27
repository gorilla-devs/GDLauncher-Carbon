/* eslint-disable solid/no-innerhtml */
import { useParams, useRouteData } from "@solidjs/router";
import { Match, Show, Switch, createEffect, createSignal } from "solid-js";
import { Dropdown, Skeleton } from "@gd/ui";
import { rspc } from "@/utils/rspcClient";
import fetchData from "../modpack.overview";
import { FEModResponse } from "@gd/core_module/bindings";

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
      const changelogQuery = rspc.createQuery(() => [
        "modplatforms.curseforgeGetModFileChangelog",
        {
          modId: modpackId,
          fileId: (fileId() as number) || (lastFile()?.id as number),
        },
      ]);
      setChangelog(changelogQuery.data?.data);
    }
  });

  return (
    <div>
      <Show when={routeData.modpackDetails.data}>
        <Dropdown
          options={(
            routeData.modpackDetails.data as FEModResponse
          ).data.latestFiles
            .reverse()
            .map((file) => ({
              key: file.id,
              label: file.displayName,
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
