import { Trans } from "@gd/i18n";
import { useRouteData } from "@solidjs/router";
import { For, onMount } from "solid-js";
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

const Java = () => {
  const [defaultJavas, setDefaultJavas] = createStore<DefaultJavasObj[]>([]);
  const routeData = useRouteData();
  const javas = () => routeData?.data?.data;

  onMount(() => {
    Object.entries(javas()).forEach((java) => {
      const javaObj = java[1];
      const defaultId = javaObj.default_id;
      const javaDefaultVersion = java[1]?.java.find((j) => j.id === defaultId);
      const newObj = javaDefaultVersion;
      newObj.majorVersion = java[0];
      console.log("TEST", java[0]);
      setDefaultJavas((prev) => [...prev, newObj]);
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
                      <div class="flex justify-around py-5 px-6 bg-shade-9 rounded-md">
                        <p class="m-0">Java {j?.version}</p>
                        <p class="m-0">{j?.path}</p>
                        <p class="m-0">{j?.type}</p>
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
