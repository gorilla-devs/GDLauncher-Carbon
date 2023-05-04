const Skeleton = () => {
  return <div />;
};

Skeleton.sidebarInstance = () => {
  return (
    <div class="flex gap-2">
      <div class="w-10 h-10 rounded-lg bg-darkSlate-700" />
      <div class="flex flex-col space-between gap-2">
        <div class="w-32 h-4 rounded-md bg-darkSlate-700" />
        <div class="w-32 h-4 rounded-md bg-darkSlate-700" />
      </div>
    </div>
  );
};

export { Skeleton };
