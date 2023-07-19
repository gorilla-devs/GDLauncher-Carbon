import { For, Show, createEffect, createSignal } from "solid-js";
import {
  CFFEModAuthor,
  FEModResponse,
  MRFEProject,
  MRFETeamMember,
  MRFETeamResponse,
} from "@gd/core_module/bindings";
import { rspc } from "@/utils/rspcClient";

type Props = {
  modpackDetails: FEModResponse | MRFEProject | undefined;
  isCurseforge: boolean;
  isModrinth: boolean;
};

const Authors = (props: Props) => {
  const [authors, setAuthors] = createSignal<MRFETeamResponse>([]);

  createEffect(() => {
    if (
      props.modpackDetails &&
      props.isModrinth &&
      (props.modpackDetails as MRFEProject)?.team
    ) {
      const modrinthAuthorsQuery = rspc.createQuery(() => [
        "modplatforms.modrinth.getTeam",
        (props.modpackDetails as MRFEProject)?.team,
      ]);

      if (modrinthAuthorsQuery.data) setAuthors(modrinthAuthorsQuery.data);
    }
  });

  const getAuthors = () => {
    if (props.isCurseforge && props.modpackDetails) {
      const modpack = props.modpackDetails as FEModResponse;
      return modpack.data.authors;
    } else if (props.isModrinth) return authors();

    return [];
  };

  return (
    <Show when={getAuthors().length > 0}>
      <div class="flex gap-2 items-center h-full">
        <div class="i-ri:user-fill" />
        <For each={getAuthors()}>
          {(author) => (
            <p class="m-0">
              {props.isCurseforge
                ? (author as CFFEModAuthor).name
                : (author as MRFETeamMember).user.username}
            </p>
          )}
        </For>
      </div>
    </Show>
  );
};

export default Authors;
