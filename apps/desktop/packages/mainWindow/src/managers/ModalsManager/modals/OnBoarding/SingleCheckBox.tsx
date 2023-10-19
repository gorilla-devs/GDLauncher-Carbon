import { Checkbox } from "@gd/ui";
import { Accessor, Setter, createEffect, createSignal } from "solid-js";

interface Props {
  title?: string;
  setList?: (list: Setter<never[]>) => void;
  setInstance?: (instance: string | undefined) => void;
  isSingleInstance?: boolean;
}

const SingleCheckBox = (props: Props) => {
  const [checked, setChecked] = createSignal(false);

  createEffect(() => {
    if (props.isSingleInstance && props.setInstance) {
      if (checked()) {
        props.setInstance(props.title);
      } else {
        props.setInstance("");
      }
      return;
    }
    if (props.setList) {
      if (checked()) {
        props.setList((list: any) => [...list, props.title]);
      } else {
        props.setList((list: any) =>
          list.filter((e: any) => e !== props.title)
        );
      }
    }
  });

  return (
    <Checkbox title={props.title} checked={checked()} onChange={setChecked} />
  );
};
export default SingleCheckBox;
