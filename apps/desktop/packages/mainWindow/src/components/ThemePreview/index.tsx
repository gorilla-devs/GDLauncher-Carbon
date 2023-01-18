import { mergeProps } from "solid-js";

interface Props {
  shade1?: string;
  shade2?: string;
  shade3?: string;
}

const ThemePreview = (props: Props) => {
  const mergedProps = mergeProps(
    {
      shade1: "#15181E",
      shade2: "#1D2028",
      shade3: "#272B35",
    },
    props
  );

  return (
    <svg
      width="100%"
      viewBox="0 0 1512 830"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      <g clip-path="url(#clip0_345_14344)">
        <rect width="1512" height="830" rx="8" fill={mergedProps.shade1} />
        <path
          d="M265 122C265 115.373 270.373 110 277 110H1043C1049.63 110 1055 115.373 1055 122V830H265V122Z"
          fill={mergedProps.shade2}
        />
        <rect
          x="286"
          y="134"
          width="742"
          height="317"
          rx="12"
          fill={mergedProps.shade3}
        />
        <rect
          x="286"
          y="511"
          width="152"
          height="152"
          rx="12"
          fill={mergedProps.shade3}
        />
        <rect
          x="286"
          y="675"
          width="120"
          height="19"
          rx="2"
          fill={mergedProps.shade3}
        />
        <rect
          x="286"
          y="698"
          width="76"
          height="16"
          rx="2"
          fill={mergedProps.shade3}
        />
        <rect
          x="622"
          y="511"
          width="152"
          height="152"
          rx="12"
          fill={mergedProps.shade3}
        />
        <rect
          x="622"
          y="675"
          width="120"
          height="19"
          rx="2"
          fill={mergedProps.shade3}
        />
        <rect
          x="622"
          y="698"
          width="76"
          height="16"
          rx="2"
          fill={mergedProps.shade3}
        />
        <rect
          x="454"
          y="511"
          width="152"
          height="152"
          rx="12"
          fill={mergedProps.shade3}
        />
        <rect
          x="454"
          y="675"
          width="120"
          height="19"
          rx="2"
          fill={mergedProps.shade3}
        />
        <rect
          x="454"
          y="698"
          width="76"
          height="16"
          rx="2"
          fill={mergedProps.shade3}
        />
        <rect
          x="790"
          y="511"
          width="152"
          height="152"
          rx="12"
          fill={mergedProps.shade3}
        />
        <rect
          x="790"
          y="675"
          width="120"
          height="19"
          rx="2"
          fill={mergedProps.shade3}
        />
        <rect
          x="790"
          y="698"
          width="76"
          height="16"
          rx="2"
          fill={mergedProps.shade3}
        />
        <path
          d="M958 523C958 516.373 963.373 511 970 511H1055V663H970C963.373 663 958 657.627 958 651V523Z"
          fill={mergedProps.shade3}
        />
        <path
          d="M958 677C958 675.895 958.895 675 960 675H1055V694H960C958.895 694 958 693.105 958 692V677Z"
          fill={mergedProps.shade3}
        />
        <rect
          x="958"
          y="698"
          width="78.4255"
          height="16"
          rx="2"
          fill={mergedProps.shade3}
        />
        <path
          d="M1072 126C1072 117.163 1079.16 110 1088 110H1476C1484.84 110 1492 117.163 1492 126V830H1072V126Z"
          fill={mergedProps.shade2}
        />
        <path
          d="M286 766C286 759.373 291.373 754 298 754H426C432.627 754 438 759.373 438 766V830H286V766Z"
          fill={mergedProps.shade3}
        />
        <path
          d="M454 766C454 759.373 459.373 754 466 754H594C600.627 754 606 759.373 606 766V830H454V766Z"
          fill={mergedProps.shade3}
        />
        <path
          d="M622 766C622 759.373 627.373 754 634 754H762C768.627 754 774 759.373 774 766V830H622V766Z"
          fill={mergedProps.shade3}
        />
        <path
          d="M790 766C790 759.373 795.373 754 802 754H930C936.627 754 942 759.373 942 766V830H790V766Z"
          fill={mergedProps.shade3}
        />
        <path
          d="M958 766C958 759.373 963.373 754 970 754H1055V830H958V766Z"
          fill={mergedProps.shade3}
        />
        <rect y="30" width="1512" height="60" fill={mergedProps.shade2} />
        <rect y="90" width="242" height="740" fill={mergedProps.shade2} />
        <rect
          x="20"
          y="770"
          width="202"
          height="40"
          rx="20"
          fill={mergedProps.shade3}
        />
        <rect
          x="20"
          y="111"
          width="190"
          height="46"
          rx="23"
          fill={mergedProps.shade3}
        />
        <rect
          x="70"
          y="249"
          width="112"
          height="16"
          rx="2"
          fill={mergedProps.shade3}
        />
        <rect
          x="70"
          y="271"
          width="72"
          height="14"
          rx="2"
          fill={mergedProps.shade3}
        />
        <rect
          x="20"
          y="247"
          width="40"
          height="40"
          rx="8"
          fill={mergedProps.shade3}
        />
        <rect
          x="70"
          y="369"
          width="112"
          height="16"
          rx="2"
          fill={mergedProps.shade3}
        />
        <rect
          x="70"
          y="391"
          width="72"
          height="14"
          rx="2"
          fill={mergedProps.shade3}
        />
        <rect
          x="20"
          y="367"
          width="40"
          height="40"
          rx="8"
          fill={mergedProps.shade3}
        />
        <rect
          x="70"
          y="189"
          width="112"
          height="16"
          rx="2"
          fill={mergedProps.shade3}
        />
        <rect
          x="70"
          y="211"
          width="72"
          height="14"
          rx="2"
          fill={mergedProps.shade3}
        />
        <rect
          x="20"
          y="187"
          width="40"
          height="40"
          rx="8"
          fill={mergedProps.shade3}
        />
        <rect
          x="70"
          y="309"
          width="112"
          height="16"
          rx="2"
          fill={mergedProps.shade3}
        />
        <rect
          x="70"
          y="331"
          width="72"
          height="14"
          rx="2"
          fill={mergedProps.shade3}
        />
        <rect
          x="20"
          y="307"
          width="40"
          height="40"
          rx="8"
          fill={mergedProps.shade3}
        />
        <rect
          x="70"
          y="429"
          width="112"
          height="16"
          rx="2"
          fill={mergedProps.shade3}
        />
        <rect
          x="70"
          y="451"
          width="72"
          height="14"
          rx="2"
          fill={mergedProps.shade3}
        />
        <rect
          x="20"
          y="427"
          width="40"
          height="40"
          rx="8"
          fill={mergedProps.shade3}
        />
        <rect width="1512" height="30" fill={mergedProps.shade1} />
      </g>
      <defs>
        <clipPath id="clip0_345_14344">
          <rect width="1512" height="830" rx="8" fill="white" />
        </clipPath>
      </defs>
    </svg>
  );
};

export default ThemePreview;
