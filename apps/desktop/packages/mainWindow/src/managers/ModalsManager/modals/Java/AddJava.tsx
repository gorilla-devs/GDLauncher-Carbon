import { Trans } from "@gd/i18n";
import { ModalProps } from "@/managers/ModalsManager";
import ModalLayout from "@/managers/ModalsManager/ModalLayout";
import { Button, Input } from "@gd/ui";

const AddJava = (props: ModalProps) => {
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
              <Input value={"adoptOpenJDK"} />
            </div>
          </div>
          <div class="flex w-full justify-end">
            <Button rounded={false} loading={true} percentage={0}>
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
