/* eslint-disable i18next/no-literal-string */
import { StepsProps } from ".";
import forgeIcon from "/assets/images/icons/forge.png";

const firstStep = (props: StepsProps) => {
  return (
    <div>
      firstStep
      <button
        onClick={() => {
          props?.nextStep?.();
        }}
      >
        NEXT
      </button>
    </div>
  );
};

export default firstStep;
