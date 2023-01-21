import { useRouteData } from "@solidjs/router";
import { For, createEffect, onMount } from "solid-js";
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
      <div class="flex flex-col gap-2">
        <For each={defaultJavas}>
          {(java) => (
            <div class="flex flex-col justify-start">
              <h3 class="mt-0 mb-4">{java?.majorVersion}</h3>
              <div class="flex justify-around py-5	px-6 bg-shade-9 rounded-md">
                <p class="m-0">Java {java?.version}</p>
                <p class="m-0">{java?.path}</p>
                <p class="m-0">{java?.type}</p>
              </div>
            </div>
          )}
        </For>
        <For each={Object.entries(javas())}>
          {(javas) => (
            <div class="flex flex-col justify-start">
              <h3 class="mt-0 mb-4">{javas[0]}</h3>
              <div class="flex flex-col gap-2">
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
  );
};

export default Java;
