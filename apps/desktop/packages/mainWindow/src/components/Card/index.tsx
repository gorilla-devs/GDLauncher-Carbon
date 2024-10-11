import { JSX, mergeProps } from "solid-js";
import vanillaIcon from "/assets/images/icons/vanilla.png";
import magicBallIcon from "/assets/images/icons/magicBall.png";
import clockIcon from "/assets/images/icons/clock.png";
import pickAxeIcon from "/assets/images/icons/pickaxe.png";
import mapIcon from "/assets/images/icons/map.png";
import questIcon from "/assets/images/icons/quest.png";

type Icon = "vanilla" | "book" | "cart" | "clock" | "pickaxe" | "sign";

interface Props {
  icon?: Icon;
  title: string;
  text: JSX.Element | string;
  class?: string;
}

const Card = (props: Props) => {
  const mergedProps = mergeProps({ title: "", text: "" }, props);

  const getIcon = (icon: Icon) => {
    switch (icon) {
      case "vanilla":
        return vanillaIcon;
      case "book":
        return magicBallIcon;
      case "cart":
        return questIcon;
      case "clock":
        return clockIcon;
      case "pickaxe":
        return pickAxeIcon;
      case "sign":
        return mapIcon;
      default:
        return vanillaIcon;
    }
  };

  return (
    <div
      class={`flex items-center gap-2 p-5 h-23 min-w-max bg-darkSlate-700 rounded-xl box-border ${
        props.class || ""
      }`}
    >
      <div class="flex justify-center items-center rounded-lg h-13 w-13">
        <img src={getIcon(props.icon || "vanilla")} class="h-10 w-10" />
      </div>
      <div>
        <p class="m-0 text-lightSlate-50 font-bold text-xl whitespace-nowrap">
          {mergedProps.text}
        </p>
        <h5 class="m-0 text-darkSlate-50 font-medium uppercase">
          {mergedProps.title}
        </h5>
      </div>
    </div>
  );
};

export default Card;
