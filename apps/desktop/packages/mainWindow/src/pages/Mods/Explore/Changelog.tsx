/* eslint-disable solid/no-innerhtml */
import { useParams, useRouteData } from "@solidjs/router";
import {
  Match,
  Show,
  Suspense,
  Switch,
  createEffect,
  createSignal
} from "solid-js";
import { Dropdown, Skeleton } from "@gd/ui";
import { rspc } from "@/utils/rspcClient";
import fetchData from "../mods.overview";
import {
  CFFEFile,
  CFFEFileIndex,
  FEModResponse
} from "@gd/core_module/bindings";
import { sortArrayByGameVersion } from "@/utils/mods";

const Changelog = () => {
  const params = useParams();
  const rspcContext = rspc.useContext();

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
      if (routeData.modrinthProjectVersions.data) {
        setFileId(routeData.modrinthProjectVersions.data[0].id);

        setOptions(
          routeData.modrinthProjectVersions.data.map((file) => ({
            key: file.id,
            label: file.version_number
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
          label: file.filename
        }))
      );
    }
  });

  createEffect(async () => {
    const modpackId = parseInt(params.id, 10);

    if (routeData.isCurseforge) {
      if (
        fileId() !== undefined ||
        (lastFile() && (lastFile() as CFFEFile).id !== undefined)
      ) {
        try {
          const changelogQuery = await rspcContext.client.query([
            "modplatforms.curseforge.getModFileChangelog",
            {
              modId: modpackId,
              fileId:
                parseInt(fileId() as string, 10) || (lastFile() as CFFEFile).id
            }
          ]);
          setChangelog(changelogQuery.data);
        } catch (e) {
          console.error(e);
        }
      }
    }
  });

  createEffect(async () => {
    if (!routeData.isCurseforge) {
      if (fileId() !== undefined) {
        try {
          const changelogQuery = await rspcContext.client.query([
            "modplatforms.modrinth.getVersion",
            fileId() as string
          ]);

          if (changelogQuery?.changelog) {
            setChangelog(changelogQuery.changelog);
          }
        } catch (err) {
          console.error(err);
        }
      }
    }
  });

  return (
    <Suspense fallback={<Skeleton.modpackChangelogPage />}>
      <div>
        <Show
          when={routeData.modpackDetails.data}
          fallback={<div>Loading...</div>}
        >
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
    </Suspense>
  );
};

export default Changelog;
