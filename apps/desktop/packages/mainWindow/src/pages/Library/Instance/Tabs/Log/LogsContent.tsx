const LogsContent = () => {
  return (
    <div class="flex-1 overflow-y-scroll">
      {" "}
      <For each={new Array(100).fill(0)}>
        {(_, index) => <div>Hello world this is a line</div>}
      </For>
      This is the last line
    </div>
  );
};

export default LogsContent;
