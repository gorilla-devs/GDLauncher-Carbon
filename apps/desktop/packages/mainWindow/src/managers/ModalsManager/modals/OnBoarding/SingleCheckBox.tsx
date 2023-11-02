import { Checkbox } from "@gd/ui";
import { Setter, createEffect, createSignal } from "solid-js";
import { instances } from "./SingleEntity";

interface Props {
  title?: string;
  setList?: (_list: Setter<never[]>) => void;
  setInstance?: (_instance: string | undefined) => void;
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
    <Checkbox
      title={props.title}
      checked={instances().find((e: any) => e === props.title)}
      onChange={setChecked}
    />
  );
};
export default SingleCheckBox;
