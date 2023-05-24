import { Trans } from "@gd/i18n";
import { ModalProps } from "@/managers/ModalsManager";
import ModalLayout from "@/managers/ModalsManager/ModalLayout";
import { Button, Dropdown } from "@gd/ui";
import { rspc } from "@/utils/rspcClient";
import { Show, createEffect, createSignal, onMount } from "solid-js";
import {
  FEManagedJavaArch,
  FEManagedJavaOs,
  FEManagedJavaVersion,
  FEVendor,
} from "@gd/core_module/bindings";
import { hasKey } from "@/utils/helpers";

type mappedOS = {
  [name: string]: FEManagedJavaOs;
};
type mappedARCH = {
  [name: string]: FEManagedJavaArch;
};

const osMappedNames: mappedOS = {
  win32: "Windows",
  darwin: "MacOs",
  linux: "Linux",
};

const archMappedNames: mappedARCH = {
  x64: "X64",
  arm64: "Aarch64",
};

const AddJava = (props: ModalProps) => {
  let javaVendors = rspc.createQuery(() => ["java.getManagedVendors"]);

  const [vendor, setVendor] = createSignal<FEVendor>("Azul");
  const [currentOs, setCurrentOs] = createSignal<{
    platform: FEManagedJavaOs | undefined;
    arch: FEManagedJavaArch | undefined;
  }>({ platform: undefined, arch: undefined });
  const [javaVersions, setJavaVersions] = createSignal<FEManagedJavaVersion[]>(
    []
  );

  // eslint-disable-next-line solid/reactivity
  let versionsByVendor = rspc.createQuery(() => [
    "java.getManagedVersionsByVendor",
    vendor(),
  ]);

  onMount(() => {
    window.getCurrentOS().then((currentOs) => {
      setCurrentOs({
        platform: osMappedNames[currentOs.platform],
        arch: archMappedNames[currentOs.arch],
      });
    });
  });

  createEffect(() => {
    const platform = currentOs().platform;
    const arch = currentOs().arch;
    if (versionsByVendor.data && platform && arch) {
      if (hasKey(versionsByVendor.data, platform)) {
        const javaVersions = versionsByVendor.data[platform][arch];
        setJavaVersions(javaVersions);
      }
    } else setJavaVersions([]);
  });

  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title}>
      <div class="flex items-center h-full flex-col justify-center">
        <div class="flex flex-col w-100 gap-8">
          <div class="flex flex-col gap-4">
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
                  onChange={(javaVendor) => {
                    setVendor(javaVendor.key as FEVendor);
                  }}
                  containerClass="border-1 border-solid border-darkSlate-600 rounded-lg"
                  options={
                    javaVendors?.data?.map((vendors) => ({
                      key: vendors as string,
                      label: vendors as string,
                    })) || []
                  }
                />
              </Show>
            </div>
            <div class="flex justify-between items-center gap-4 w-full">
              <h5 class="m-0 w-30">
                <Trans
                  key="java.java_major"
                  options={{
                    defaultValue: "Java Major",
                  }}
                />
              </h5>
              <Show
                when={!versionsByVendor.isLoading && javaVersions().length > 0}
              >
                <Dropdown
                  containerClass="border-1 border-solid border-darkSlate-600 rounded-lg w-full"
                  options={
                    javaVersions()?.map((versions) => ({
                      key: versions.id as string,
                      label: versions.name as string,
                    })) || []
                  }
                />
              </Show>
              <Show when={javaVersions().length === 0}>
                <Trans
                  key="java.no_available_javas"
                  options={{
                    defaultValue: "No java available for this vendor",
                  }}
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
