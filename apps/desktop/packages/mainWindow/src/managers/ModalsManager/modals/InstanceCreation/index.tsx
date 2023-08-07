import { Tab, TabList, TabPanel, Tabs } from "@gd/ui";
import { ModalProps } from "../..";
import ModalLayout from "../../ModalLayout";
import { Trans } from "@gd/i18n";
import Custom from "./Custom";
import Import from "./Import";

const InstanceCreation = (props: ModalProps) => {
  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      overflowHiddenDisabled={true}
      noPadding={true}
    >
      <div class="flex flex-col justify-between scrollbar-hide w-120 overflow-y-scroll h-136">
        <Tabs variant="block" paddingY="py-2">
          <TabList>
            <Tab>
              <Trans key="instance.instance_creation_custom_tab" />
            </Tab>
            <Tab>
              <Trans key="instance.instance_import_tab" />
            </Tab>
          </TabList>
          <TabPanel>
            <Custom data={props.data} />
          </TabPanel>
          <TabPanel>
            <Import />
          </TabPanel>
        </Tabs>
      </div>
    </ModalLayout>
  );
};

export default InstanceCreation;
