import { mergeProps } from "solid-js";

interface Props {
  percentage: number | string;
  color?: string;
}

const Progressbar = (props: Props) => {
  const mergedProps = mergeProps({ percentage: 0 }, props);

  return (
    <div class="w-full bg-darkSlate-900 m-0 mx-auto rounded-lg overflow-hidden border border-gray-300">
      <div
        class={`${
          props.color ? props.color : "bg-green-500"
        } text-xs leading-none py-1`}
        style={{ width: `${mergedProps.percentage}%` }}
      />
    </div>
  );
};

export { Progressbar };
