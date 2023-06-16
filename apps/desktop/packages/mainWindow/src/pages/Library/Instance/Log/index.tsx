import { logsObj, setLogsObj } from "@/utils/logs";
import { port } from "@/utils/rspcClient";
import { Trans } from "@gd/i18n";
import { useParams, useRouteData } from "@solidjs/router";
import { For, Match, Switch, createEffect, createResource } from "solid-js";
import fetchData from "../instance.logs.data";
import { getRunningState } from "@/utils/instances";

const fetchLogs = async (logId: number) => {
  return fetch(`http://localhost:${port}/instance/log?id=${logId}`);
};

const Logs = () => {
  const params = useParams();

  const routeData = useRouteData<typeof fetchData>();

  const instanceId = () => parseInt(params.id, 10);

  const instanceLogs = () =>
    routeData.logs.data
      ?.reverse()
      .find((item) => item.instance_id === instanceId());

  const logId = () => instanceLogs()?.id;

  const [allLogs, { refetch }] = createResource(logId, fetchLogs);

  async function streamToJson(readableStream: ReadableStream) {
    // Get the reader from the stream
    // eslint-disable-next-line no-undef
    let reader: ReadableStreamDefaultReader;
    // Check if the stream is locked
    if (readableStream.locked) {
      // Stream is locked, use the existing reader
      if (readableStream.locked) {
        throw new Error("ReadableStream is locked but no reader found");
      }
      reader = readableStream.getReader();
    } else {
      // Stream is not locked, get a new reader
      reader = readableStream.getReader();
    }

    // Read the stream
    let result = "";
    while (true) {
      const { value, done } = await reader.read();

      if (done) {
        // Stream has ended
        break;
      }

      // Add value to the result
      result += new TextDecoder("utf-8").decode(value);
    }

    // Fix and parse broken JSON
    const fixedJson = "[" + result.replace(/}{/g, "},{") + "]";
    const json = JSON.parse(fixedJson);
    if (!readableStream.locked) readableStream.cancel();
    return json;
  }

  createEffect(() => {
    if (routeData.instanceDetails.data) {
      const isRunning = getRunningState(routeData.instanceDetails.data.state);
      if (isRunning) {
        refetch();
      }
    }
  });

  createEffect(() => {
    if (allLogs()?.body) {
      streamToJson((allLogs() as any).body).then((logs) => {
        setLogsObj(instanceId(), () => {
          return logs;
        });
      });
    }
  });

  const instanceLogss = () => logsObj[instanceId()] || [];

  return (
    <div class="overflow-y-auto pb-4 max-h-full divide-y divide-darkSlate-500">
      <Switch>
        <Match when={(instanceLogss().length || 0) > 0}>
          <For each={instanceLogss()}>
            {(log) => {
              return (
                <div class="flex flex-col justify-center items-center w-full">
                  <pre class="m-0 w-full box-border py-2 leading-8 whitespace-pre-wrap pl-4">
                    <code class="text-darkSlate-50 text-md">{log?.line}</code>
                  </pre>
                </div>
              );
            }}
          </For>
        </Match>
        <Match when={(instanceLogss().length || 0) === 0}>
          <div class="h-full flex justify-center items-center">
            <p>
              <Trans
                key="logs.no_logs"
                options={{
                  defaultValue: "No logs",
                }}
              />
            </p>
          </div>
        </Match>
      </Switch>
    </div>
  );
};

export default Logs;
