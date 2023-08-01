import { Progressbar } from "@gd/ui";
import JavaLogo from "/assets/images/icons/java-logo.svg";
import { Trans } from "@gd/i18n";

const percentage = 40;

const AutomaticStep = () => {
  return (
    <div class="flex flex-col items-center h-50 w-110 justify-around">
      <div class="flex flex-col items-center">
        <img src={JavaLogo} class="h-16 w-16" />
        <h3>
          <Trans
            key="java.java_missing"
            options={{
              defaultValue: "Java {{version}} missing",
              version: 8,
            }}
          />
        </h3>
      </div>
      <Progressbar percentage={percentage} />
      <p class="mb-0">
        {`${percentage}%`}
        <Trans
          key="java.automatic_download_progress"
          options={{
            defaultValue: "Downloaded",
          }}
        />
      </p>
    </div>
  );
};

export default AutomaticStep;
