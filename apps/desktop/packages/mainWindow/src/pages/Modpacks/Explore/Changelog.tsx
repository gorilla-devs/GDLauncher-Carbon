/* eslint-disable solid/no-innerhtml */
import { useParams, useRouteData } from "@solidjs/router";
import { Match, Show, Switch, createEffect, createSignal } from "solid-js";
import { Dropdown, Skeleton } from "@gd/ui";
import { rspc } from "@/utils/rspcClient";
import fetchData from "../modpack.overview";
import {
  CFFEFile,
  CFFEFileIndex,
  FEModResponse,
  MRFEProject,
  MRFEVersion,
} from "@gd/core_module/bindings";
import { sortArrayByGameVersion } from "@/utils/Mods";

const Changelog = () => {
  const params = useParams();

  const routeData: ReturnType<typeof fetchData> = useRouteData();
  const lastFile = () =>
    routeData.isCurseforge &&
    routeData.modpackDetails?.data?.data.latestFiles[
      routeData.modpackDetails?.data?.data.latestFiles.length - 1
    ];

  const [options, setOptions] = createSignal<{ key: string; label: string }[]>(
    []
  );
  const [fileId, setFileId] = createSignal<number | string | undefined>(
    undefined
  );
  const [changeLog, setChangelog] = createSignal<string | undefined>(undefined);

  createEffect(() => {
    if (!routeData.modpackDetails.data) return;
    if (!routeData.isCurseforge) {
      const query = rspc.createQuery(() => [
        "modplatforms.modrinth.getVersions",
        (routeData.modpackDetails.data as MRFEProject).versions,
      ]);

      if (query.data) {
        const sortedVersions = sortArrayByGameVersion(
          query.data as MRFEVersion[]
        ) as MRFEVersion[];

        setFileId(sortedVersions[0].id);

        setOptions(
          sortedVersions.map((file) => ({
            key: file.id,
            label: file.version_number,
          }))
        );
      }
    } else {
      const sortedVersions = sortArrayByGameVersion(
        (routeData.modpackDetails.data as FEModResponse)?.data
          .latestFilesIndexes
      );
      setOptions(
        (sortedVersions as CFFEFileIndex[]).map((file) => ({
          key: file.fileId.toString(),
          label: file.filename,
        }))
      );
    }
  });

  createEffect(() => {
    const modpackId = parseInt(params.id, 10);

    if (routeData.isCurseforge)
      if (
        fileId() !== undefined ||
        (lastFile() && (lastFile() as CFFEFile).id !== undefined)
      ) {
        // eslint-disable-next-line solid/reactivity
        const changelogQuery = rspc.createQuery(() => [
          "modplatforms.curseforge.getModFileChangelog",
          {
            modId: modpackId,
            fileId: (fileId() as number) || (lastFile() as CFFEFile).id,
          },
        ]);
        setChangelog(changelogQuery.data?.data);
      }
  });

  createEffect(() => {
    if (!routeData.isCurseforge) {
      if (fileId() !== undefined) {
        // eslint-disable-next-line solid/reactivity
        const changelogQuery = rspc.createQuery(() => [
          "modplatforms.modrinth.getVersion",
          fileId() as string,
        ]);

        if (changelogQuery.data?.changelog) {
          setChangelog(changelogQuery.data.changelog);
        }
      }
    }
  });

  return (
    <div>
      <Show when={routeData.modpackDetails.data}>
        <Dropdown
          options={options()}
          onChange={(fileId) => {
            setFileId(fileId.key);
          }}
        />
      </Show>
      <Switch fallback={<Skeleton.modpackChangelogPage />}>
        <Match when={changeLog() && routeData.isCurseforge}>
          <div innerHTML={changeLog()} />
        </Match>
        <Match when={changeLog() && !routeData.isCurseforge}>
          <pre>{changeLog()}</pre>
        </Match>
      </Switch>
    </div>
  );
};

export default Changelog;
