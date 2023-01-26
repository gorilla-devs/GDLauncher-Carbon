import { queryClient, rspc } from "@/utils/rspcClient";
import { Trans } from "@gd/i18n";
import { Java as JavaType } from "@gd/native_interface/bindings";
import { Checkbox } from "@gd/ui";
import { useRouteData } from "@solidjs/router";
import { For, createEffect } from "solid-js";
import { createStore } from "solid-js/store";

interface JavaObj {
  id: string;
  path: string;
  type: string;
  version: string;
}
interface DefaultJavasObj extends JavaObj {
  majorVersion: string;
}

interface RouteData {
  data: {
    data: { [key: number]: JavaType };
  };
}

const Java = () => {
  const [defaultJavas, setDefaultJavas] = createStore<DefaultJavasObj[]>([]);
  const routeData: RouteData = useRouteData();

  let mutation = rspc.createMutation(["java.setDefault"], {
    onMutate: (newTheme) => {
      queryClient.setQueryData(["java.setDefault", null], newTheme);
    },
  });

  createEffect(() => {
    const javasData = () => routeData?.data;
    const javas: () => { [key: number]: JavaType } = () => javasData()?.data;
    Object.entries(javas()).forEach((java) => {
      const javaObj = java[1];
      const defaultId = javaObj.default_id;
      const javaDefaultVersion = java[1]?.java.find((j) => j.id === defaultId);
      let newObj: any = javaDefaultVersion;
      newObj.majorVersion = java[0];
      setDefaultJavas([newObj]);
    });
  });

  return (
    <div class="bg-shade-8 w-full h-auto flex flex-col py-5	px-6 box-border">
      <h2 class="m-0 mb-7 text-4">
        <Trans
          key="java"
          options={{
            defaultValue: "Java",
          }}
        />
      </h2>
      <div class="flex flex-col">
        <div class="flex flex-col gap-4 mb-10 border-2 border-solid border-shade-7 p-4">
          <For each={defaultJavas}>
            {(java) => (
              <div class="flex flex-col justify-start">
                <h4 class="mt-0 mb-4 text-shade-5">{java?.majorVersion}</h4>
                <div class="flex justify-around py-5	px-6 bg-shade-9 rounded-md">
                  <p class="m-0">
                    <Trans
                      key="java"
                      options={{
                        defaultValue: "Java",
                      }}
                    />
                    {java?.version}
                  </p>
                  <p class="m-0">{java?.path}</p>
                  <p class="m-0">{java?.type}</p>
                </div>
              </div>
            )}
          </For>
        </div>
        <div class="flex flex-col gap-4 border-2 border-solid border-shade-7 p-4">
          <For each={Object.entries(javas())}>
            {(javas) => (
              <div class="flex flex-col justify-start">
                <h5 class="mt-0 mb-4">{javas[0]}</h5>
                <div class="flex flex-col gap-4">
                  <For each={javas[1].java as []}>
                    {(j) => (
                      <div class="flex justify-between py-5 px-6 bg-shade-9 rounded-md">
                        <p class="m-0">
                          <Trans
                            key="java"
                            options={{
                              defaultValue: "Java",
                            }}
                          />
                          {j?.version}
                        </p>
                        <p class="m-0 text-ellipsis max-w-[245px] overflow-hidden">
                          {j?.path}
                        </p>
                        <p class="m-0">{j?.type}</p>
                        <Checkbox
                          onChange={() => {
                            mutation.mutate({
                              major_version: parseInt(javas[0], 10),
                              id: j?.id,
                            });
                          }}
                        />
                      </div>
                    )}
                  </For>
                </div>
              </div>
            )}
          </For>
        </div>
      </div>
    </div>
  );
};

export default Java;
