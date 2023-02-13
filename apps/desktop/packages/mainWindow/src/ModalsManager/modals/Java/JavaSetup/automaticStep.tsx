/* eslint-disable i18next/no-literal-string */
import { Progressbar } from "@gd/ui";
import { StepsProps } from ".";
import JavaLogo from "/assets/images/icons/java-logo.svg";

const AutomaticStep = (props: StepsProps) => {
  return (
    <div class="w-110 flex flex-col items-center h-50 justify-around">
      <div class="flex flex-col items-center">
        <img src={JavaLogo} class="h-16 w-16" />
        <h3>Java 8 missing</h3>
      </div>
      <Progressbar percentage={40} />
    </div>
  );
};

export default AutomaticStep;
