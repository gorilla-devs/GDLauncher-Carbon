import { children, Show } from "solid-js";

interface Props {
  children: HTMLElement | string;
  class?: string;
  type?: string;
  disabled?: boolean;
  icon?: Element | any;
  iconRight?: boolean;
  /* eslint-disable no-unused-vars */
  onClick?: (e: MouseEvent) => void;
}

const getStateClasses = (type: string, disabled: boolean) => {
  const isDisabled = () => disabled;
  const isPrimary = () => type === "primary" || !type;
  const isOutline = () => type === "outline";
  const isSecondary = () => type === "secondary";
  const isGlow = () => type === "glow";

  let classes = [];

  if (isPrimary()) {
    if (!isDisabled()) {
      classes.push("bg-accent-main hover:bg-accent-hover");
    } else {
      classes.push("text-black-semiblack");
    }
  } else if (isOutline()) {
    if (!isDisabled()) {
      classes.push(
        "border-white border-1 hover:border-accent-hover hover:text-accent-hover text-black-semiblack"
      );
    } else {
      classes.push(
        "border-1 bg-black-black border-black-semiblack text-black-semiblack"
      );
    }
  } else if (isSecondary()) {
    if (!isDisabled()) {
      classes.push("border-1 hover:border-white border-black-semiblack");
    } else {
      classes.push("border-1 border-black-semiblack");
    }
  } else if (isGlow()) {
    if (!isDisabled()) {
      classes.push(
        "shadow-md shadow-accent-main bg-accent-main hover:shadow-lg hover:bg-accent-hover"
      );
    } else
      classes.push("bg-black-gray text-black-lightGray text-black-semiblack");
  }

  return classes.join(" ");
};

function Button(props: Props) {
  const c = children(() => props.children);

  return (
    <div
      class={`transition duration-200 ease-in-out font-main max-w-max py-4 px-8 rounded-full cursor-pointer uppercase font-bold flex gap-2 ${
        props.class || ""
      } ${
        props.disabled ? "bg-black-black text-black-gray" : "text-white"
      }  ${getStateClasses(props.type || "", !!props.disabled)}`}
      onClick={(e) => props.onClick?.(e)}
    >
      <Show when={props.icon}>{props.icon}</Show>
      {c()}
    </div>
  );
}

export { Button };
