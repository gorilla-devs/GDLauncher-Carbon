interface Props {
  value?: string;
  icon?: string;
  onClick?: () => void;
  disabled?: boolean;
}

export const DeviceCode = (props: Props) => {
  return (
    <div class="h-13 bg-shade-8 flex justify-center items-center text-white font-bold gap-2 rounded-md opacity-100 w-47 font-ubuntu">
      <span
        class="text-2xl font-normal"
        style={{
          color: props.disabled ? "#404759" : "",
        }}
      >
        {props.value}
      </span>
      <span
        class="cursor-pointer text-shade-0 i-ri:file-copy-fill"
        style={{
          color: props.disabled ? "#404759" : "#8A8B8F",
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
