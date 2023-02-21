/* eslint-disable i18next/no-literal-string */
import { Tab, TabList, TabPanel, Tabs } from "@gd/ui";
import { ModalProps } from "..";
import ModalLayout from "../ModalLayout";

const LogViewer = (props: ModalProps) => {
  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title} noPadding>
      <div class="h-130 w-190">
        <div class="bg-shade-8">
          <Tabs variant="traditional">
            <TabList>
              <Tab>TEST</Tab>
              <Tab>TEST</Tab>
              <Tab>TEST</Tab>
              <Tab>TEST</Tab>
            </TabList>
            <div class="bg-shade-7">
              <TabPanel>1</TabPanel>
              <TabPanel>2</TabPanel>
              <TabPanel>3</TabPanel>
            </div>
          </Tabs>
        </div>
      </div>
    </ModalLayout>
  );
};

export default LogViewer;
