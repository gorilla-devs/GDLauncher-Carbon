import { Trans } from "@gd/i18n";
import SiderbarWrapper from "./wrapper";

const Sidebar = () => {
  return (
    <SiderbarWrapper collapsable={false}>
      <Trans
        key="hello"
        options={{
          defaultValue: "hello",
        }}
      />
    </SiderbarWrapper>
  );
};

export default Sidebar;
