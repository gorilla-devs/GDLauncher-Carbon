export interface Props {
  checked?: boolean;
  disabled?: boolean;
  /* eslint-disable no-unused-vars */
  onChange?: (e: Event) => void;
}

function Switch(props: Props) {
  return (
    <label class="relative inline-block w-10 h-5 m-2">
      <input class="opacity-0 w-0 h-0 peer" type="checkbox" />
      <span
        class="absolute cursor-pointer top-0 left-0 right-0 bottom-0 transition-all duration-100 ease-in-out rounded-full before:absolute before:content-[] before:w-4	before:h-4 before:left-0.5 before:bottom-0.5 before:bg-white before:rounded-full peer-checked:before:translate-x-5 before:transition-all before:ease-in-out before:duration-100"
        classList={{
          "bg-primary": true,
        }}
      />
    </label>
  );
}

export { Switch };
