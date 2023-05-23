import { Trans } from "@gd/i18n";
import { ModalProps } from "@/managers/ModalsManager";
import ModalLayout from "@/managers/ModalsManager/ModalLayout";
import { Button, Dropdown, Input } from "@gd/ui";
import { rspc } from "@/utils/rspcClient";
import { Show } from "solid-js";

const AddJava = (props: ModalProps) => {
  let javaVendors = rspc.createQuery(() => ["java.getManagedVendors"]);

  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title}>
      <div class="flex items-center h-full flex-col justify-center">
        <div class="flex flex-col max-w-90 gap-8">
          <div class="flex flex-col gap-4">
            <div class="flex justify-between items-center gap-4">
              <h5 class="m-0">
                <Trans
                  key="java.java_major"
                  options={{
                    defaultValue: "Java Major",
                  }}
                />
              </h5>
              <Input value={16} />
            </div>
            <div class="flex justify-between items-center gap-4">
              <h5 class="m-0">
                <Trans
                  key="java.java_distribution"
                  options={{
                    defaultValue: "Distribution",
                  }}
                />
              </h5>
              <Show when={!javaVendors.isLoading}>
                <Dropdown
                  options={
                    javaVendors?.data?.map((vendors) => ({
                      key: vendors as string,
                      label: vendors as string,
                    })) || []
                  }
                />
              </Show>
            </div>
          </div>
          <div class="flex w-full justify-end">
            <Button rounded={false}>
              <Trans
                key="java.install"
                options={{
                  defaultValue: "Install",
                }}
              />
            </Button>
          </div>
        </div>
      </div>
    </ModalLayout>
  );
};

export default AddJava;
