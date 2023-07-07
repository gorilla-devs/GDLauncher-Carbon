import { Button, Tab, TabList, TabPanel, Tabs } from "@gd/ui";
import { ModalProps } from "../..";
import ModalLayout from "../../ModalLayout";
import { Trans, useTransContext } from "@gd/i18n";
import { onMount } from "solid-js";
import { rspc } from "@/utils/rspcClient";
import Custom from "./Custom";

const InstanceCreation = (props: ModalProps) => {
  const [t] = useTransContext();

  const scanImportableInstancesMutation = rspc.createMutation([
    "instance.scanImportableInstances",
  ]);

  onMount(() => {
    scanImportableInstancesMutation.mutate("LegacyGDLauncher");
  });

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
            <div class="p-5">
              <Button>
                <div class="i-ri:folder-open-fill" />
                <Trans key="instance.import_instance" />
              </Button>
            </div>
          </TabPanel>
        </Tabs>
      </div>
    </ModalLayout>
  );
};

export default InstanceCreation;
