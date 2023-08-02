/* eslint-disable solid/no-innerhtml */
import { rspc } from "@/utils/rspcClient";
import sanitizeHtml from "sanitize-html";
import { ModalProps } from "..";
import ModalLayout from "../ModalLayout";

const PrivacyStatement = (props: ModalProps) => {
  const body = rspc.createQuery(() => ["settings.getPrivacyStatementBody"]);

  const sanitizedHtml = () => {
    if (!body.data) return undefined;

    console.log(body.data);

    return sanitizeHtml(body.data);
  };

  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title}>
      <div class="h-130 w-190 overflow-hidden">
        <div class="overflow-y-auto max-h-full" innerHTML={sanitizedHtml()} />
      </div>
    </ModalLayout>
  );
};

export default PrivacyStatement;
