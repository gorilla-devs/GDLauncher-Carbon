// import { Trans } from "@gd/i18n";
import { ModalProps } from "@/ModalsManager";
import ModalLayout from "@/ModalsManager/ModalLayout";
import { Button, Input } from "@gd/ui";

/* eslint-disable i18next/no-literal-string */
const AddJava = (props: ModalProps) => {
  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title}>
      <div class="flex flex-col justify-center items-center h-full">
        <div class="flex flex-col max-w-90 gap-8">
          <div class="flex flex-col gap-4">
            <div class="flex justify-between items-center gap-4">
              <h5 class="m-0">
                {/* <Trans
            key="java_major"
            options={{
              defaultValue: "Java Major",
            }}
          /> */}
                Java Major
              </h5>
              <Input value={16} />
            </div>
            <div class="flex justify-between items-center gap-4">
              <h5 class="m-0">
                {/* <Trans
            key="java_distribution"
            options={{
              defaultValue: "Distribution",
            }}
          /> */}
                Distribution
              </h5>
              <Input value={"adoptOpenJDK"} />
            </div>
          </div>
          <div class="w-full flex justify-end">
            <Button rounded={false} loading={true} percentage={0}>
              Install
            </Button>
          </div>
        </div>
      </div>
    </ModalLayout>
  );
};

export default AddJava;
