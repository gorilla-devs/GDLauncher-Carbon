import { mergeProps } from "solid-js";

interface Props {
  icon?: string;
  title: string;
  text: string;
}

const Card = (props: Props) => {
  const mergedProps = mergeProps({ title: "", text: "" }, props);
  return (
    <div class="flex items-center justify-between p-5 h-23 w-59 bg-black-semiblack rounded-xl box-border">
      <div class="h-13 w-13 bg-black-black rounded-lg"></div>
      <div>
        <h5 class="text-black-lightGray uppercase font-medium m-0">
          {mergedProps.title}
        </h5>
        <p class="text-white font-bold uppercase text-2xl m-0">
          {mergedProps.text}
        </p>
      </div>
    </div>
  );
};

export default Card;
