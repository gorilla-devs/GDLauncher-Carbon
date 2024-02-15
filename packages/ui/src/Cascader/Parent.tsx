import { Portal } from "solid-js/web";
import ChildsMenu, { ChildsMenuProps } from "./ChildsMenu";

export const Parent = (props: ChildsMenuProps) => {
  return (
    <Portal mount={document.body}>
      <div id="menu-id" class="flex gap-1">
        <ChildsMenu {...props} />
      </div>
    </Portal>
  );
};
export default Parent;
