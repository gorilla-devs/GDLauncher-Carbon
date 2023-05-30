import { For } from "solid-js";

const Skeleton = () => {
  return <div class="w-1/3 min-w-20 h-4 rounded-md bg-darkSlate-500" />;
};

const SidebarInstance = () => {
  return (
    <div class="flex gap-2 py-2 px-4">
      <div class="w-10 h-10 rounded-lg bg-darkSlate-700" />
      <div class="flex flex-col space-between gap-2">
        <div class="w-32 h-4 rounded-md bg-darkSlate-700" />
        <div class="w-32 h-4 rounded-md bg-darkSlate-700" />
      </div>
    </div>
  );
};

Skeleton.sidebarInstance = SidebarInstance;

Skeleton.sidebarInstances = () => {
  return (
    <div class="flex flex-col gap-2 mt-10">
      <For each={new Array(4)}>{() => <SidebarInstance />}</For>
    </div>
  );
};

Skeleton.sidebarInstanceSmall = () => {
  return <div class="h-10 w-10 rounded-lg bg-darkSlate-700 py-2 px-4" />;
};

const Instance = () => {
  return (
    <div class="flex flex-col gap-2">
      <div class="w-38 h-38 rounded-lg bg-darkSlate-700" />
      <div class="flex flex-col space-between gap-2">
        <div class="w-32 h-4 rounded-md bg-darkSlate-700" />
        <div class="w-32 h-4 rounded-md bg-darkSlate-700" />
      </div>
    </div>
  );
};

Skeleton.instance = Instance;

Skeleton.instances = () => {
  return (
    <div class="flex gap-4">
      <For each={new Array(3)}>{() => <Instance />}</For>
    </div>
  );
};

Skeleton.news = () => {
  return <div class="w-full h-80 rounded-lg bg-darkSlate-700 mb-5" />;
};

const Modpack = () => {
  return (
    <div class="flex justify-between h-[220px] w-full rounded-xl bg-darkSlate-700 p-4 gap-4 box-border">
      <div class="rounded-xl select-none h-30 w-30 bg-darkSlate-500" />
      <div class="flex flex-col space-between gap-2 flex-1">
        <div class="w-full h-4 rounded-md bg-darkSlate-500" />
        <div class="w-full h-4 rounded-md bg-darkSlate-500" />
        <div class="w-1/2 h-4 rounded-md bg-darkSlate-500" />
      </div>
    </div>
  );
};

Skeleton.modpack = Modpack;

Skeleton.modpacksList = () => {
  return (
    <div class="flex flex-col gap-2 w-full">
      <Modpack />
      <Modpack />
      <Modpack />
      <Modpack />
    </div>
  );
};
const ModpackVersion = () => {
  return (
    <div class="flex flex-col justify-between w-1/2 rounded-xl p-4 gap-4 box-border">
      <div class="w-full h-2 rounded-md bg-darkSlate-500" />
      <div class="w-1/2 h-2 rounded-md bg-darkSlate-500" />
    </div>
  );
};
Skeleton.modpackVersionList = () => {
  return (
    <div class="flex flex-col gap-2 w-full">
      <ModpackVersion />
      <ModpackVersion />
      <ModpackVersion />
      <ModpackVersion />
    </div>
  );
};
Skeleton.modpackOverviewPage = () => {
  return (
    <div class="flex flex-col gap-2 w-full">
      <div class="w-1/4 h-3 rounded-xl bg-darkSlate-500 mt-5" />
      <div class="w-full h-3 rounded-xl bg-darkSlate-500" />
      <div class="w-1/4 h-3 rounded-xl bg-darkSlate-500 mt-5" />
      <div class="w-full h-3 rounded-xl bg-darkSlate-500" />
      <div class="w-1/4 h-3 rounded-xl bg-darkSlate-500 mt-5" />
      <div class="w-full h-3 rounded-xl bg-darkSlate-500" />
      <div class="mt-5 flex flex-wrap gap-4">
        <div class="w-72 h-44 rounded-xl bg-darkSlate-500" />
        <div class="w-72 h-44 rounded-xl bg-darkSlate-500" />
        <div class="w-72 h-44 rounded-xl bg-darkSlate-500" />
        <div class="w-72 h-44 rounded-xl bg-darkSlate-500" />
      </div>
    </div>
  );
};

Skeleton.modpackScreenshotsPage = () => {
  return (
    <div class="mt-5 flex flex-wrap gap-4">
      <div class="w-72 h-44 rounded-xl bg-darkSlate-500" />
      <div class="w-72 h-44 rounded-xl bg-darkSlate-500" />
      <div class="w-72 h-44 rounded-xl bg-darkSlate-500" />
      <div class="w-72 h-44 rounded-xl bg-darkSlate-500" />
    </div>
  );
};
Skeleton.modpackChangelogPage = () => {
  return (
    <div class="flex flex-col gap-2 w-full">
      <div class="w-1/4 h-3 rounded-xl bg-darkSlate-500 mt-5" />
      <div class="w-1/2 h-3 rounded-xl bg-darkSlate-500" />
      <div class="w-1/4 h-3 rounded-xl bg-darkSlate-500 mt-5" />
      <div class="w-1/2 h-3 rounded-xl bg-darkSlate-500" />
      <div class="w-1/4 h-3 rounded-xl bg-darkSlate-500 mt-5" />
      <div class="w-1/2 h-3 rounded-xl bg-darkSlate-500" />
      <div class="w-1/4 h-3 rounded-xl bg-darkSlate-500 mt-5" />
      <div class="w-1/2 h-3 rounded-xl bg-darkSlate-500" />
      <div class="w-1/4 h-3 rounded-xl bg-darkSlate-500 mt-5" />
      <div class="w-1/2 h-3 rounded-xl bg-darkSlate-500" />
      <div class="w-1/4 h-3 rounded-xl bg-darkSlate-500 mt-5" />
      <div class="w-1/2 h-3 rounded-xl bg-darkSlate-500" />
      <div class="w-1/4 h-3 rounded-xl bg-darkSlate-500 mt-5" />
      <div class="w-1/2 h-3 rounded-xl bg-darkSlate-500" />
      <div class="w-1/4 h-3 rounded-xl bg-darkSlate-500 mt-5" />
      <div class="w-1/2 h-3 rounded-xl bg-darkSlate-500" />
    </div>
  );
};

Skeleton.modpackSidebarCategories = () => {
  return (
    <div class="flex flex-col gap-4 w-full py-2 mt-4">
      <For each={new Array(16)}>
        {() => (
          <div class="flex gap-2 items-center">
            <div class="w-5 h-5 rounded-xl bg-darkSlate-500" />
            <div class="w-1/2 h-3 rounded-xl bg-darkSlate-500" />
          </div>
        )}
      </For>
    </div>
  );
};

Skeleton.select = () => {
  return <div class="w-31	h-12 rounded-full bg-darkSlate-500" />;
};

export { Skeleton };