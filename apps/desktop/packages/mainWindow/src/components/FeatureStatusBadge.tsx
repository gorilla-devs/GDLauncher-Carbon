import { Trans } from "@gd/i18n";

type Props = {
  type: "new" | "beta" | "soon";
};

export default function FeatureStatusBadge(props: Props) {
  const colors = () => {
    switch (props.type) {
      case "new":
        return "bg-green-400 text-lightSlate-50";
      case "beta":
        return "bg-yellow-400 text-darkSlate-900";
      case "soon":
        return "bg-primary-400 text-lightSlate-50";
    }
  };

  const text = () => {
    switch (props.type) {
      case "new":
        return <Trans key="badge_new" />;
      case "beta":
        return <Trans key="badge_beta" />;
      case "soon":
        return <Trans key="badge_soon" />;
    }
  };

  return (
    <div
      class={`flex items-center justify-center rounded-md px-2 py-1 text-center font-bold text-[0.6rem] uppercase ${colors()}`}
    >
      {text()}
    </div>
  );
}
