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
            <div class="flex items-center">
              <TabList>
                <Tab>
                  <div class="w-full flex gap-3 items-center justify-start">
                    <div class="w-10 h-10 rounded-xl bg-green" />
                    <p>Valhelsia</p>
                  </div>
                </Tab>
                <Tab>
                  <div class="w-full flex gap-3 items-center justify-start">
                    <div class="w-10 h-10 rounded-xl bg-green" />
                    <p>Valhelsia</p>
                  </div>
                </Tab>
                <Tab>
                  <div class="w-full flex gap-3 items-center justify-start">
                    <div class="w-10 h-10 rounded-xl bg-green" />
                    <p>Valhelsia</p>
                  </div>
                </Tab>
                <Tab>
                  <div class="w-full flex gap-3 items-center justify-start">
                    <div class="w-10 h-10 rounded-xl bg-green" />
                    <p>Valhelsia</p>
                  </div>
                </Tab>
              </TabList>
              <div class="flex gap-4 px-5">
                <div class="i-ri:upload-2-line text-shade-0 cursor-pointer" />
                <div class="i-ri:file-copy-fill text-shade-0 cursor-pointer" />
              </div>
            </div>
            <div class="bg-shade-7">
              <TabPanel>1</TabPanel>
              <TabPanel>2</TabPanel>
              <TabPanel>3</TabPanel>
              <TabPanel>4</TabPanel>
            </div>
          </Tabs>
        </div>
      </div>
    </ModalLayout>
  );
};

export default LogViewer;
