type Props = {
  value?: string;
  icon?: string;
};

export const CodeInput = (props: Props) => {
  return (
    <div class="h-13 w-45 bg-[#1D2028] opacity-100 rounded-md flex justify-center items-center">
      {props.value} <span class="i-gdl:copy"></span>
      {/* <span class="i-hero:archive-box"></span> */}
    </div>
  );
};
