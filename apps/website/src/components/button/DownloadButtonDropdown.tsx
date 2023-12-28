import { DownloadItems } from "../../consts";
import Button from "./Button";
import { AiOutlineArrowDown } from "solid-icons/ai";

export const DownloadButtonDropdown = () => {
  return (
    <Button
      items={DownloadItems}
      isDropdown={true}
      intent="primary"
      size="medium"
      class="items-center"
    >
      <span>DOWNLOAD FOR</span>
      <AiOutlineArrowDown size={20} />
    </Button>
  );
};
