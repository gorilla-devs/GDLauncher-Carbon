/* eslint-disable solid/no-innerhtml */
import { rspc } from "@/utils/rspcClient";
import sanitizeHtml from "sanitize-html";
import { ModalProps } from "..";
import ModalLayout from "../ModalLayout";
import { Suspense } from "solid-js";

const PrivacyStatement = (props: ModalProps) => {
  const body = rspc.createQuery(() => ({
    queryKey: ["settings.getPrivacyStatementBody"]
  }));

  const sanitizedHtml = () => {
    if (!body.data) return undefined;
    return sanitizeHtml(body.data);
  };

  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title}>
      <Suspense>
        <div class="h-130 w-190 overflow-hidden">
          <div class="overflow-y-auto max-h-full" innerHTML={sanitizedHtml()} />
        </div>
      </Suspense>
    </ModalLayout>
  );
};

export default PrivacyStatement;
