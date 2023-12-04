import { Checkbox } from "@gd/ui";
import { Show, createSignal } from "solid-js";

interface Props {
  canExpand?: boolean;
  title?: string;
  onChange?: (_checked: boolean) => void;
  indeterminate?: boolean;
  checked?: boolean;
}

const ExportCheckbox = (props: Props) => {
  const [expand, setExpand] = createSignal(false);
  return (
    <div class="flex items-center gap-2 h-10 px-2">
      <Show when={props.canExpand === true}>
        <div
          class={`${expand() ? "i-ep:arrow-down" : "i-ep:arrow-up"}`}
          onClick={() => {
            setExpand(!expand());
          }}
        ></div>
      </Show>
      <Checkbox
        children={<span class="text-sm">{props.title}</span>}
        checked={props.checked}
        onChange={() => {}}
        indeterminate={props.indeterminate}
      />
    </div>
  );
};
export default ExportCheckbox;
