import { DownloadItems } from "../../consts";
import Button from "./Button";

export const DownloadButtonDropdown = () => {
  return (
    <Button
      items={DownloadItems}
      isDropdown={true}
      intent="primary"
      size="medium"
    >
      DOWNLOAD FOR
    </Button>
  );
};
