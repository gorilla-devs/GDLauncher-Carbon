interface Props {
  value?: string;
  icon?: string;
  onClick?: () => void;
  disabled?: boolean;
  id?: string;
}

export const DeviceCode = (props: Props) => {
  return (
    <div class="h-13 flex justify-center items-center text-white font-bold gap-2 rounded-md bg-darkSlate-800 opacity-100 w-47 font-ubuntu">
      <span
        class="text-2xl font-normal"
        style={{
          color: props.disabled ? "#404759" : ""
        }}
      >
        {props.value}
      </span>
      <span
        id={props.id}
        class="text-darkSlate-50 i-ri:file-copy-fill hover:bg-lightSlate-50 transition-color duration-100 ease-in-out"
        style={{
          color: props.disabled ? "#404759" : "#8A8B8F"
        }}
        onClick={() => {
          if (props?.onClick && !props.disabled) {
            props?.onClick();
          }
        }}
      />
    </div>
  );
};
