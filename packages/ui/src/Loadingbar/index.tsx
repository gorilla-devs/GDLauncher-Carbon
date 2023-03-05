type Props = {
  class?: string;
};

export const LoadingBar = (props: Props) => {
  return (
    <div
      class={`h-2 bg-shade-5 w-full overflow-hidden rounded-full ${
        props.class || ""
      }`}
    >
      <div class="w-full h-full origin-[0%_50%] animate-loadingbar bg-primary" />
    </div>
  );
};
