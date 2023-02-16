import { mergeProps } from "solid-js";
import vanillaIcon from "/assets/images/icons/vanilla.png";
import bookIcon from "/assets/images/icons/book.png";
import clockIcon from "/assets/images/icons/clock.png";
import pickAxeIcon from "/assets/images/icons/pickaxe.png";
import signIcon from "/assets/images/icons/sign.png";
import cartIcon from "/assets/images/icons/cart.png";

type Icon = "vanilla" | "book" | "cart" | "clock" | "pickaxe" | "sign";

interface Props {
  icon?: Icon;
  title: string;
  text: string;
  class?: string;
}

const Card = (props: Props) => {
  const mergedProps = mergeProps({ title: "", text: "" }, props);

  const getIcon = (icon: Icon) => {
    switch (icon) {
      case "vanilla":
        return vanillaIcon;
      case "book":
        return bookIcon;
      case "cart":
        return cartIcon;
      case "clock":
        return clockIcon;
      case "pickaxe":
        return pickAxeIcon;
      case "sign":
        return signIcon;
      default:
        return vanillaIcon;
    }
  };

  return (
    <div
      class={`flex items-center justify-between p-5 h-23 w-59 bg-shade-7 rounded-xl box-border ${
        props.class || ""
      }`}
    >
      <div class="bg-shade-8 flex justify-center items-center h-13 w-13 rounded-lg">
        <img src={getIcon(props.icon || "vanilla")} />
      </div>
      <div>
        <h5 class="m-0 text-shade-0 uppercase font-medium">
          {mergedProps.title}
        </h5>
        <p class="text-white uppercase m-0 font-bold text-2xl">
          {mergedProps.text}
        </p>
      </div>
    </div>
  );
};

export default Card;
