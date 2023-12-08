import { ModalProps } from "..";
import ModalLayout from "../ModalLayout";
import { Trans } from "@gd/i18n";

const WhyAreAdsNeeded = (props: ModalProps) => {
  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props.title}
      height="h-120"
      width="w-160"
    >
      <div class="flex flex-col justify-between h-full overflow-y-auto text-white">
        <p>
          <Trans key="ads.paragraph-1" />
        </p>
        <p class="mt-2">
          <Trans key="ads.paragraph-2" />
        </p>
        <ul class="list-disc pl-5">
          <li>
            <Trans key="ads.paragraph-2-list-element-1" />
          </li>
          <li>
            <Trans key="ads.paragraph-2-list-element-2" />
          </li>
          <li>
            <Trans key="ads.paragraph-2-list-element-3" />
          </li>
        </ul>
        <p class="mt-2">
          <Trans key="ads.paragraph-3" />
        </p>
        <ul class="list-disc pl-5 mt-2">
          <li>
            <Trans key="ads.paragraph-3-list-element-1" />
          </li>
          <li>
            <Trans key="ads.paragraph-3-list-element-2" />
          </li>
          <li>
            <Trans key="ads.paragraph-3-list-element-3" />
          </li>
        </ul>
        <p class="mt-2">
          <Trans key="ads.paragraph-4" />
        </p>
        <p class="mt-2">
          <Trans key="ads.paragraph-5" />
        </p>
        <p class="mt-2">
          <Trans key="ads.paragraph-6" />
        </p>
        <p class="mt-2">
          <Trans key="ads.paragraph-7" />
        </p>
        <p class="mt-4 italic">
          <Trans key="ads.paragraph-8" />
        </p>
      </div>
    </ModalLayout>
  );
};

export default WhyAreAdsNeeded;
