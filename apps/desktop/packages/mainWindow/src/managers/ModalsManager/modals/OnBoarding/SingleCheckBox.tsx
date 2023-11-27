import { Checkbox } from "@gd/ui";
import { Setter } from "solid-js";
import { instances } from "./SingleEntity";

interface Props {
  title?: string;
  setList?: Setter<never[]>;
  setInstance?: (_instance: string | undefined) => void;
  isSingleInstance?: boolean;
  indeterminate?: boolean;
  isNested?: boolean;
}

const SingleCheckBox = (props: Props) => {
  const handleChange = () => {
    if (!props.isNested) {
      if (instances().some((instance) => instance === props.title)) {
        if (props.setList) {
          props.setList((prev: any) =>
            prev.filter((e: any) => e !== props.title)
          );
        }
      } else {
        if (props.setList) {
          props.setList((prev: any) => [...prev, props.title] as never);
        }
      }
    } else {
    }
  };
  return (
    <Checkbox
      children={<span class="text-sm">{props.title}</span>}
      checked={instances().some((instance) => instance === props.title)}
      onChange={handleChange}
      indeterminate={props.indeterminate}
    />
  );
};

export default SingleCheckBox;
