import { Trans } from "@gd/i18n";
import { ModalProps, useModal } from "@/managers/ModalsManager";
import ModalLayout from "@/managers/ModalsManager/ModalLayout";
import { Button, Dropdown, createNotification } from "@gd/ui";
import { rspc } from "@/utils/rspcClient";
import { Show, createEffect, createSignal, onMount } from "solid-js";
import {
  FEManagedJavaArch,
  FEManagedJavaOs,
  FEManagedJavaVersion,
  FEVendor
} from "@gd/core_module/bindings";
import { hasKey } from "@/utils/helpers";

type mappedOS = {
  [name: string]: FEManagedJavaOs;
};

const osMappedNames: mappedOS = {
  win32: "windows",
  darwin: "macOs",
  linux: "linux"
};

const AddManagedJava = (props: ModalProps) => {
  let javaVendors = rspc.createQuery(() => ({
    queryKey: ["java.getManagedVendors"]
  }));

  const [vendor, setVendor] = createSignal<FEVendor>("azul");
  const [currentOs, setCurrentOs] = createSignal<{
    platform: FEManagedJavaOs | undefined;
    arch: FEManagedJavaArch | undefined;
  }>({ platform: undefined, arch: undefined });
  const [javaVersions, setJavaVersions] = createSignal<FEManagedJavaVersion[]>(
    []
  );
  const [selectedJavaVersion, setSelectedJavaVersion] = createSignal("");
  const [loading, setLoading] = createSignal(false);
  const modalsContext = useModal();
  const addNotification = createNotification();

  // eslint-disable-next-line solid/reactivity
  let versionsByVendor = rspc.createQuery(() => ({
    queryKey: ["java.getManagedVersionsByVendor", vendor()]
  }));

  let addJavaMutation = rspc.createMutation(() => ({
    mutationKey: ["java.setupManagedJava"]
  }));

  onMount(() => {
    window.getCurrentOS().then((currentOs) => {
      setCurrentOs({
        platform: osMappedNames[currentOs.platform],
        arch: currentOs.arch as FEManagedJavaArch
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

  const mappedJavaVersions = () =>
    javaVersions()?.map((versions) => ({
      key: versions.id as string,
      label: versions.name as string
    })) || [];

  const mappedVendors = () =>
    javaVendors?.data?.map((vendors) => ({
      key: vendors as string,
      label: vendors as string
    })) || [];

  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title}>
      <div class="flex items-center h-full flex-col justify-center">
        <div class="flex flex-col w-100 gap-8">
          <div class="flex flex-col gap-4">
            <div class="flex justify-between items-center gap-4">
              <h5 class="m-0">
                <Trans
                  key="java.java_vendors"
                  options={{
                    defaultValue: "Vendors"
                  }}
                />
              </h5>
              <Show when={!javaVendors.isLoading}>
                <Dropdown
                  onChange={(javaVendor) => {
                    setVendor(javaVendor.key as FEVendor);
                  }}
                  containerClass="border-1 border-solid border-darkSlate-600 rounded-lg"
                  options={mappedVendors()}
                />
              </Show>
            </div>
            <div class="flex justify-between items-center gap-4 w-full">
              <h5 class="m-0 w-30">
                <Trans
                  key="java.java_major"
                  options={{
                    defaultValue: "Java Major"
                  }}
                />
              </h5>
              <Show
                when={!versionsByVendor.isLoading && javaVersions().length > 0}
              >
                <Dropdown
                  onChange={(version) =>
                    setSelectedJavaVersion(version.key as string)
                  }
                  containerClass="border-1 border-solid border-darkSlate-600 rounded-lg w-full"
                  options={mappedJavaVersions()}
                />
              </Show>
              <Show
                when={
                  javaVersions().length === 0 && !versionsByVendor.isLoading
                }
              >
                <Trans
                  key="java.no_available_javas"
                  options={{
                    defaultValue: "No java available for this vendor"
                  }}
                />
              </Show>
            </div>
          </div>
          <div class="flex w-full justify-end">
            <Button
              rounded={false}
              loading={loading()}
              onClick={async () => {
                const id = selectedJavaVersion() || mappedJavaVersions()[0].key;
                const vend = vendor() || mappedVendors()[0].key;

                if (currentOs().arch && currentOs().platform && id && vend) {
                  try {
                    setLoading(true);

                    await addJavaMutation.mutateAsync({
                      arch: currentOs().arch as FEManagedJavaArch,
                      os: currentOs().platform as FEManagedJavaOs,
                      id,
                      vendor: vend
                    });

                    addNotification("Java added successfully");
                  } catch (err) {
                    console.error(err);
                    modalsContext?.closeModal();
                    setLoading(false);
                  }
                }
              }}
            >
              <Trans
                key="java.install"
                options={{
                  defaultValue: "Install"
                }}
              />
            </Button>
          </div>
        </div>
      </div>
    </ModalLayout>
  );
};

export default AddManagedJava;
