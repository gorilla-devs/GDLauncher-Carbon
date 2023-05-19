/* eslint-disable i18next/no-literal-string */
import { Tab, TabList, TabPanel, Tabs } from "@gd/ui";
import { ModalProps } from "..";
import ModalLayout from "../ModalLayout";
import { For, createEffect, createResource } from "solid-js";
import { port, rspc } from "@/utils/rspcClient";
import { getRunningState } from "@/utils/instances";
import { UngroupedInstance } from "@gd/core_module/bindings";
import { streamToJson } from "@/utils/helpers";
import { createStore } from "solid-js/store";

type RunningInstances = {
  [instanceId: number]: UngroupedInstance | undefined;
};

const fetchLogs = async (logId: number) => {
  const log = await fetch(`http://localhost:${port}/instance/log?id=${logId}`);
  if (log.body) {
    const parsed = await streamToJson(log.body);
    console.log("LOGS PARSED", parsed);

    return parsed;
  }
};

const LogViewer = (props: ModalProps) => {
  const instances = rspc.createQuery(() => ["instance.getInstancesUngrouped"]);
  const [activeInstances, setActiveInstances] = createStore<RunningInstances>(
    {}
  );

  createEffect(() => {
    instances?.data?.forEach((instance: UngroupedInstance) => {
      const isRunning = getRunningState(instance.status);

      setActiveInstances((prev) => {
        const newState: RunningInstances = { ...prev };

        if (isRunning) {
          if (!newState[instance.id]) {
            newState[instance.id] = instance;
          }
        } else {
          if (newState[instance.id]) {
            newState[instance.id] = undefined;
          }
        }

        return newState;
      });
    });
  });

  const runningInstances: () => UngroupedInstance[] = () =>
    Object.values(activeInstances).filter(
      (running) => running
    ) as UngroupedInstance[];

  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title} noPadding>
      <div class="h-130 w-190 overflow-hidden">
        <div class="bg-darkSlate-800 max-h-full">
          <Tabs variant="traditional">
            <div class="flex items-center max-h-full">
              <TabList>
                <For each={runningInstances()}>
                  {(instance) => {
                    const runningState = getRunningState(instance.status);

                    const [logs] = createResource(
                      runningState?.log_id,
                      fetchLogs
                    );

                    console.log("RUNNING-LOGS", logs());

                    return (
                      <Tab>
                        <div class="w-full flex gap-2 items-center h-10 justify-start">
                          <div class="w-6 rounded-md bg-green h-6" />
                          <p class="my-2">{instance.name}</p>
                        </div>
                      </Tab>
                    );
                  }}
                </For>
              </TabList>
              <div class="flex gap-4 px-5">
                <div class="cursor-pointer text-darkSlate-50 i-ri:upload-2-line" />
                <div class="text-darkSlate-50 cursor-pointer i-ri:file-copy-fill" />
              </div>
            </div>
            <div class="bg-darkSlate-700 overflow-y-auto max-h-130">
              <TabPanel>
                <div class="overflow-y-auto divide-y divide-darkSlate-500">
                  {/* <For each={logs()}>
                    {(log) => (
                      <div class="flex flex-col justify-center items-center">
                        <pre class="m-0 leading-8 whitespace-pre-wrap pl-4">
                          {log.data}
                        </pre>
                      </div>
                    )}
                  </For> */}
                </div>
              </TabPanel>
              <TabPanel>2</TabPanel>
              <TabPanel>3</TabPanel>
              <TabPanel>4</TabPanel>
            </div>
          </Tabs>
        </div>
      </div>
    </ModalLayout>
  );
};

export default LogViewer;
