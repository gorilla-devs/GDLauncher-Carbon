interface Props {
  title: string;
  modloader: string;
  version: string;
  onClick?: (_e: MouseEvent) => void;
}

const Tile = (props: Props) => {
  return (
    <div
      class="instance-tile flex flex-col justify-center items-start cursor-pointer snap-start"
      // onClick={(e) => props?.onClick?.(e)}
    >
      <div class="h-38 w-38 bg-green-600 rounded-2xl" />
      <h4 class="my-2">{props.title}</h4>
      <div class="flex justify-between text-shade-0">
        <p class="m-0">{props.modloader}</p>
        <p class="m-0">{props.version}</p>
      </div>
    </div>
  );
};

export default Tile;
