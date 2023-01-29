import { queryClient, rspc } from "@/utils/rspcClient";
import { Trans } from "@gd/i18n";
import { Java as JavaType } from "@gd/native_interface/bindings";
import { Button } from "@gd/ui";
import { useNavigate, useRouteData } from "@solidjs/router";
import { For, Show, createEffect, createSignal } from "solid-js";
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
  const [defaultJavasIds, setDefaultJavasIds] = createSignal<string[]>([]);
  const routeData: RouteData = useRouteData();
  const javasData = () => routeData?.data;
  const javas: () => { [key: number]: JavaType } = () => javasData()?.data;
  const navigate = useNavigate();

  let mutation = rspc.createMutation(["java.setDefault"], {
    onMutate: (newTheme) => {
      queryClient.setQueryData(["java.setDefault", null], newTheme);
    },
  });

  createEffect(() => {
    Object.entries(javas()).forEach((java) => {
      const javaObj = java[1];
      const defaultId = javaObj.default_id;
      const javaDefaultVersion = java[1]?.java.find((j) => j.id === defaultId);
      let newObj: any = javaDefaultVersion;
      setDefaultJavasIds((prev) => [...prev, defaultId]);
      newObj.majorVersion = java[0];
      setDefaultJavas((prev) => {
        const filteredPrev = prev.filter((j) => j.id !== newObj.id);
        return [...filteredPrev, newObj];
      });
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
        <div class="flex justify-between mb-4">
          <h5 class="m-0 flex items-center">
            <Trans
              key="all_versions"
              options={{
                defaultValue: "All versions",
              }}
            />
          </h5>
          <Button
            rounded={false}
            variant="secondary"
            onClick={() => {
              navigate("?m=addjava");
            }}
          >
            <div class="i-ri:add-fill text-shade-5 text-xl" />
          </Button>
        </div>
        <div class="flex flex-col gap-4 border-2 border-solid border-shade-7 p-4">
          <For each={Object.entries(javas())}>
            {(javas) => (
              <div class="flex flex-col justify-start">
                <h5 class="mt-0 mb-4 text-shade-5">{javas[0]}</h5>
                <div class="flex flex-col gap-4">
                  <For each={javas[1].java as []}>
                    {(j: JavaObj) => (
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

                        <div
                          class="cursor-pointer"
                          onChange={() => {
                            mutation.mutate({
                              major_version: parseInt(javas[0], 10),
                              id: j?.id,
                            });
                          }}
                        >
                          <Show
                            when={defaultJavasIds().includes(j?.id)}
                            fallback={
                              <div class="i-ri:star-line text-shade-5 text-xl" />
                            }
                          >
                            <div class="i-ri:star-fill text-yellow text-xl" />
                          </Show>
                        </div>
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
