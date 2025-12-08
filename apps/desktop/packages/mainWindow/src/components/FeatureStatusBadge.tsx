import { Trans } from "@gd/i18n"

interface Props {
  type: "new" | "beta" | "soon"
}

export default function FeatureStatusBadge(props: Props) {
  const colors = () => {
    switch (props.type) {
      case "new":
        return "bg-green-400 text-lightSlate-50"
      case "beta":
        return "bg-yellow-900 text-lightSlate-50"
      case "soon":
        return "bg-primary-400 text-lightSlate-50"
    }
  }

  const text = () => {
    switch (props.type) {
      case "new":
        return <Trans key="general:_trn_badge_new" />
      case "beta":
        return <Trans key="general:_trn_badge_beta" />
      case "soon":
        return <Trans key="general:_trn_badge_soon" />
    }
  }

  return (
    <div
      class={`flex items-center justify-center rounded px-1.5 text-center text-[0.55rem] font-bold uppercase ${colors()}`}
    >
      {text()}
    </div>
  )
}
