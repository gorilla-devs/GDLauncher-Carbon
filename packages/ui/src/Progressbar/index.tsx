import { mergeProps } from "solid-js";

interface Props {
  percentage: number;
}

const Progressbar = (props: Props) => {
  const mergedProps = mergeProps({ percentage: 0 }, props);

  return (
    <div class="w-full bg-gray-200 max-w-sm my-12 mx-auto rounded-lg overflow-hidden border border-gray-300">
      <div
        class="bg-green-500 text-xs leading-none py-1"
        style={{ width: `${mergedProps.percentage}%` }}
      />
    </div>
  );
};

export { Progressbar };
