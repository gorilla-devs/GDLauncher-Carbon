type Props = {
  value?: string;
  icon?: string;
  onClick?: () => void;
};

export const CodeInput = (props: Props) => {
  return (
    <div class="h-13 w-45 bg-black-black opacity-100 rounded-md flex justify-center items-center text-white font-bold  font-ubuntu gap-4">
      <span class="text-2xl">{props.value}</span>
      <span
        class="i-gdl:copy cursor-pointer"
        onClick={() => {
          if (props?.onClick) {
            props?.onClick();
          }
        }}
      />
    </div>
  );
};
