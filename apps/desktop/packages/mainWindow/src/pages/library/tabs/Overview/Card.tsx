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
    <div class="flex items-center justify-between p-5 h-23 w-59 bg-black-semiblack rounded-xl box-border">
      <div class="h-13 w-13 bg-black-black rounded-lg flex justify-center items-center">
        <img src={getIcon(props.icon || "vanilla")} />
      </div>
      <div>
        <h5 class="text-shade-0 uppercase font-medium m-0">
          {mergedProps.title}
        </h5>
        <p class="text-white font-bold uppercase text-2xl m-0">
          {mergedProps.text}
        </p>
      </div>
    </div>
  );
};

export default Card;
