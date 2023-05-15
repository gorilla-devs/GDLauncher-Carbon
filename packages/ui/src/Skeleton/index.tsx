import { For } from "solid-js";

const Skeleton = () => {
  return <div />;
};

Skeleton.sidebarInstance = () => {
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

export { Skeleton };
