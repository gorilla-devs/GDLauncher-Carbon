import { JSX, mergeProps } from "solid-js"
import vanillaIcon from "/assets/images/icons/vanilla.png"
import magicBallIcon from "/assets/images/icons/magicBall.png"
import clockIcon from "/assets/images/icons/clock.png"
import pickAxeIcon from "/assets/images/icons/pickaxe.png"
import mapIcon from "/assets/images/icons/map.png"
import questIcon from "/assets/images/icons/quest.png"

type Icon = "vanilla" | "book" | "cart" | "clock" | "pickaxe" | "sign"

interface Props {
  icon?: Icon
  title: string
  text: JSX.Element | string
  class?: string
}

const Card = (props: Props) => {
  const mergedProps = mergeProps({ title: "", text: "" }, props)

  const getIcon = (icon: Icon) => {
    switch (icon) {
      case "vanilla":
        return vanillaIcon
      case "book":
        return magicBallIcon
      case "cart":
        return questIcon
      case "clock":
        return clockIcon
      case "pickaxe":
        return pickAxeIcon
      case "sign":
        return mapIcon
      default:
        return vanillaIcon
    }
  }

  return (
    <div
      data-gdl="instance.overview.card"
      class={`h-23 bg-darkSlate-700 box-border flex min-w-max items-center gap-2 rounded-xl p-5 ${
        props.class || ""
      }`}
    >
      <div class="h-13 w-13 flex items-center justify-center rounded-lg">
        <img
          data-gdl="instance.overview.card.icon"
          src={getIcon(props.icon || "vanilla")}
          class="h-10 w-10"
        />
      </div>
      <div>
        <p
          data-gdl="instance.overview.card.value"
          class="text-lightSlate-50 m-0 whitespace-nowrap text-xl font-bold"
        >
          {mergedProps.text}
        </p>
        <h5
          data-gdl="instance.overview.card.label"
          class="text-lightSlate-700 m-0 font-medium uppercase"
        >
          {mergedProps.title}
        </h5>
      </div>
    </div>
  )
}

export default Card
