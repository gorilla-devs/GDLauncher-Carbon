/* eslint-disable i18next/no-literal-string */
import { Progressbar } from "@gd/ui";
import JavaLogo from "/assets/images/icons/java-logo.svg";

const percentage = 40;

const AutomaticStep = () => {
  return (
    <div class="w-110 flex flex-col items-center h-50 justify-around">
      <div class="flex flex-col items-center">
        <img src={JavaLogo} class="h-16 w-16" />
        <h3>Java 8 missing</h3>
      </div>
      <Progressbar percentage={percentage} />
      <p class="mb-0">{percentage}% Downloaded</p>
    </div>
  );
};

export default AutomaticStep;
