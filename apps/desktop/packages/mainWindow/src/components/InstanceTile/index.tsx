type Props = {
  title: string;
  modloader: string;
  version: string;
};

const InstanceTile = (props: Props) => {
  return (
    <div class="flex flex-col justify-center items-start">
      <div class="h-38 w-38 bg-green-600 rounded-2xl" />
      <h4 class="my-2">{props.title}</h4>
      <div class="flex justify-between text-black-lightGray">
        <p class="m-0">{props.modloader}</p>
        <p class="m-0">{props.version}</p>
      </div>
    </div>
  );
};

export default InstanceTile;
