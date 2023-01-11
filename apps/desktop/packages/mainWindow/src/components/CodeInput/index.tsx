interface Props {
  value?: string;
  icon?: string;
  onClick?: () => void;
  disabled?: boolean;
}

export const DeviceCode = (props: Props) => {
  return (
    <div class="h-13 w-47 bg-black-black opacity-100 rounded-md flex justify-center items-center text-white font-bold  font-ubuntu gap-2">
      <span
        class="text-2xl font-normal"
        style={{
          color: props.disabled ? "#404759" : "",
        }}
      >
        {props.value}
      </span>
      <span
        class="i-ri:file-copy-fill cursor-pointer text-shade-0"
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
