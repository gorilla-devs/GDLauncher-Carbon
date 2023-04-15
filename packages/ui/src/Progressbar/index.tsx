import { mergeProps } from "solid-js";

export interface Props {
  percentage: number;
}

const Progressbar = (props: Props) => {
  const mergedProps = mergeProps({ percentage: 0 }, props);

  return (
    <div class="w-full bg-darkSlate-900 max-w-sm m-0 mx-auto rounded-lg overflow-hidden border border-gray-300">
      <div
        class="bg-green-500 text-xs leading-none py-1"
        style={{ width: `${mergedProps.percentage}%` }}
      />
    </div>
  );
};

export { Progressbar };
