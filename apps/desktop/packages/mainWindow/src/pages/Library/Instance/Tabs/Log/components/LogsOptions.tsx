import RightHandSide from "@/pages/Settings/components/RightHandSide";
import Row from "@/pages/Settings/components/Row";
import RowsContainer from "@/pages/Settings/components/RowsContainer";
import { Trans } from "@gd/i18n";
import { Popover, Slider, Switch } from "@gd/ui";
import { createSignal } from "solid-js";
import { Show } from "solid-js";

type DensityRowProps = {
  selected: boolean;
  onClick: () => void;
};

function LowDensityRows(props: DensityRowProps) {
  return (
    <div
      class="relative w-24 h-10 flex flex-col justify-center items-center box-border border-2 border-solid border-darkSlate-100 rounded-md cursor-pointer"
      classList={{ "bg-lightSlate-600": props.selected }}
      onClick={props.onClick}
    >
      <div class="w-14 h-3 rounded-sm bg-darkSlate-100" />
      <Show when={props.selected}>
        <div class="absolute top-1 right-1 w-4 h-4 i-ri:checkbox-fill text-green-600" />
      </Show>
    </div>
  );
}

function MediumDensityRows(props: DensityRowProps) {
  return (
    <div
      class="relative w-24 h-10 flex flex-col gap-1 justify-center items-center box-border border-2 border-solid border-darkSlate-100 rounded-md cursor-pointer"
      classList={{ "bg-lightSlate-600": props.selected }}
      onClick={props.onClick}
    >
      <div class="w-14 h-1.5 rounded-sm bg-darkSlate-100" />
      <div class="w-14 h-1.5 rounded-sm bg-darkSlate-100" />
      <Show when={props.selected}>
        <div class="absolute top-1 right-1 w-4 h-4 i-ri:checkbox-fill text-green-600" />
      </Show>
    </div>
  );
}

function HighDensityRows(props: DensityRowProps) {
  return (
    <div
      class="relative w-24 h-10 flex flex-col gap-1 justify-center items-center box-border border-2 border-solid border-darkSlate-100 rounded-md cursor-pointer"
      classList={{ "bg-lightSlate-600": props.selected }}
      onClick={props.onClick}
    >
      <div class="w-14 h-1 rounded-sm bg-darkSlate-100" />
      <div class="w-14 h-1 rounded-sm bg-darkSlate-100" />
      <div class="w-14 h-1 rounded-sm bg-darkSlate-100" />
      <div class="w-14 h-1 rounded-sm bg-darkSlate-100" />
      <Show when={props.selected}>
        <div class="absolute top-1 right-1 w-4 h-4 i-ri:checkbox-fill text-green-600" />
      </Show>
    </div>
  );
}

type ColumnProps = {
  selected: boolean;
  onClick: () => void;
};

function TimestampColumn(props: ColumnProps) {
  return (
    <div
      class="relative w-24 h-10 flex gap-1 justify-center items-center box-border border-2 border-solid border-darkSlate-100 rounded-md cursor-pointer"
      classList={{ "bg-lightSlate-600": props.selected }}
      onClick={props.onClick}
    >
      <div class="w-4 h-2 rounded-sm bg-lightSlate-50" />
      <div class="w-4 h-2 rounded-sm bg-darkSlate-100" />
      <div class="w-4 h-2 rounded-sm bg-darkSlate-100" />
      <div class="w-10 h-2 rounded-sm bg-darkSlate-100" />
      <Show when={props.selected}>
        <div class="absolute top-1 right-1 w-4 h-4 i-ri:checkbox-fill text-green-600" />
      </Show>
    </div>
  );
}

function SourceKindColumn(props: ColumnProps) {
  return (
    <div
      class="relative w-24 h-10 flex gap-1 justify-center items-center box-border border-2 border-solid border-darkSlate-100 rounded-md cursor-pointer"
      classList={{ "bg-lightSlate-600": props.selected }}
      onClick={props.onClick}
    >
      <div class="w-4 h-2 rounded-sm bg-darkSlate-100" />
      <div class="w-4 h-2 rounded-sm bg-darkSlate-100" />
      <div class="w-10 h-2 rounded-sm bg-darkSlate-100" />
      <Show when={props.selected}>
        <div class="absolute top-1 right-1 w-4 h-4 i-ri:checkbox-fill text-green-600" />
      </Show>
    </div>
  );
}

function LoggerColumn(props: ColumnProps) {
  return (
    <div
      class="relative w-24 h-10 flex gap-1 justify-center items-center box-border border-2 border-solid border-darkSlate-100 rounded-md cursor-pointer"
      classList={{ "bg-lightSlate-600": props.selected }}
      onClick={props.onClick}
    >
      <div class="w-4 h-2 rounded-sm bg-darkSlate-100" />
      <div class="w-4 h-2 rounded-sm bg-darkSlate-100" />
      <Show when={props.selected}>
        <div class="absolute top-1 right-1 w-4 h-4 i-ri:checkbox-fill text-green-600" />
      </Show>
    </div>
  );
}

function ThreadNameColumn(props: ColumnProps) {
  return (
    <div
      class="relative w-24 h-10 flex gap-1 justify-center items-center box-border border-2 border-solid border-darkSlate-100 rounded-md cursor-pointer"
      classList={{ "bg-lightSlate-600": props.selected }}
      onClick={props.onClick}
    >
      <div class="w-4 h-2 rounded-sm bg-darkSlate-100" />
      <div class="w-4 h-2 rounded-sm bg-lightSlate-50" />
      <div class="w-4 h-2 rounded-sm bg-darkSlate-100" />
      <div class="w-10 h-2 rounded-sm bg-darkSlate-100" />
      <Show when={props.selected}>
        <div class="absolute top-1 right-1 w-4 h-4 i-ri:checkbox-fill text-green-600" />
      </Show>
    </div>
  );
}

function LogLevelColumn(props: ColumnProps) {
  return (
    <div
      class="relative w-24 h-10 flex gap-1 justify-center items-center box-border border-2 border-solid border-darkSlate-100 rounded-md cursor-pointer"
      classList={{ "bg-lightSlate-600": props.selected }}
      onClick={props.onClick}
    >
      <div class="w-4 h-2 rounded-sm bg-darkSlate-100" />
      <div class="w-4 h-2 rounded-sm bg-darkSlate-100" />
      <div class="w-4 h-2 rounded-sm bg-lightSlate-50" />
      <div class="w-10 h-2 rounded-sm bg-darkSlate-100" />
      <Show when={props.selected}>
        <div class="absolute top-1 right-1 w-4 h-4 i-ri:checkbox-fill text-green-600" />
      </Show>
    </div>
  );
}

export type LogDensity = "low" | "medium" | "high";

export type Columns = {
  timestamp: boolean;
  logger: boolean;
  sourceKind: boolean;
  threadName: boolean;
  level: boolean;
};

type Props = {
  logsDensity: LogDensity;
  setLogsDensity: (_: LogDensity) => void;
  columns: Columns;
  setColumns: (_: Columns) => void;
  fontMultiplier: 0 | 1 | 2;
  setFontMultiplier: (_: 0 | 1 | 2) => void;
  autoFollowPreference: boolean;
  setAutoFollowPreference: (_: boolean) => void;
  startLogMessageOnNewLine: boolean;
  setStartLogMessageOnNewLine: (_: boolean) => void;
};

export default function LogsOptions(props: Props) {
  const [isOpen, setIsOpen] = createSignal(false);

  return (
    <Popover
      color="bg-transparent"
      noTip
      noShadow
      trigger="click"
      onOpen={() => setIsOpen(true)}
      onClose={() => setIsOpen(false)}
      content={() => (
        <div class="text-lightSlate-50 bg-darkSlate-700 w-130 h-auto p-4 rounded-lg shadow-lg shadow-darkSlate-900">
          <RowsContainer>
            <Row>
              <div>
                <Trans key="logs_density" />
              </div>
              <RightHandSide>
                <div class="flex gap-6 w-full h-auto items-center">
                  <div class="w-full flex flex-col items-center gap-3">
                    <div class="text-sm text-lightSlate-600 text-center">
                      <Trans key="logs_density.low" />
                    </div>
                    <LowDensityRows
                      selected={props.logsDensity === "low"}
                      onClick={() => props.setLogsDensity("low")}
                    />
                  </div>
                  <div class="w-full flex flex-col items-center gap-3">
                    <div class="text-sm text-lightSlate-600 text-center">
                      <Trans key="logs_density.comfortable" />
                    </div>
                    <MediumDensityRows
                      selected={props.logsDensity === "medium"}
                      onClick={() => props.setLogsDensity("medium")}
                    />
                  </div>
                  <div class="w-full flex flex-col items-center gap-3">
                    <div class="text-sm text-lightSlate-600 text-center">
                      <Trans key="logs_density.compact" />
                    </div>
                    <HighDensityRows
                      selected={props.logsDensity === "high"}
                      onClick={() => props.setLogsDensity("high")}
                    />
                  </div>
                </div>
              </RightHandSide>
            </Row>
            <hr class="h-px w-full bg-darkSlate-400 border-0 rounded" />
            <Row>
              <div>
                <Trans key="font_size" />
              </div>
              <RightHandSide>
                <div class="w-84">
                  <Slider
                    min={0}
                    max={2}
                    steps={1}
                    value={props.fontMultiplier}
                    marks={{
                      0: {
                        label: (
                          <div class="text-lightSlate-600 text-xs">{"aA"}</div>
                        )
                      },
                      1: {
                        label: (
                          <div class="text-lightSlate-600 text-sm">{"aA"}</div>
                        )
                      },
                      2: {
                        label: (
                          <div class="text-lightSlate-600 text-base">
                            {"aA"}
                          </div>
                        )
                      }
                    }}
                    onChange={(val) => {
                      props.setFontMultiplier(val as 0 | 1 | 2);
                    }}
                  />
                </div>
              </RightHandSide>
            </Row>
            <hr class="h-px w-full bg-darkSlate-400 border-0 rounded" />
            <Row>
              <div>
                <Trans key="columns" />
              </div>
              <RightHandSide>
                <div class="flex flex-wrap gap-6 w-84 h-auto items-center">
                  <div class="w-auto flex flex-col items-center gap-3">
                    <div class="text-sm text-lightSlate-600 text-center">
                      <Trans key="columns.timestamp" />
                    </div>
                    <TimestampColumn
                      selected={props.columns.timestamp}
                      onClick={() =>
                        props.setColumns({
                          ...props.columns,
                          timestamp: !props.columns.timestamp
                        })
                      }
                    />
                  </div>
                  <div class="w-auto flex flex-col items-center gap-3">
                    <div class="text-sm text-lightSlate-600 text-center">
                      <Trans key="columns.logger" />
                    </div>
                    <LoggerColumn
                      selected={props.columns.logger}
                      onClick={() =>
                        props.setColumns({
                          ...props.columns,
                          logger: !props.columns.logger
                        })
                      }
                    />
                  </div>
                  <div class="w-auto flex flex-col items-center gap-3">
                    <div class="text-sm text-lightSlate-600 text-center">
                      <Trans key="columns.source_kind" />
                    </div>
                    <SourceKindColumn
                      selected={props.columns.sourceKind}
                      onClick={() =>
                        props.setColumns({
                          ...props.columns,
                          sourceKind: !props.columns.sourceKind
                        })
                      }
                    />
                  </div>
                  <div class="w-auto flex flex-col items-center gap-3">
                    <div class="text-sm text-lightSlate-600 text-center">
                      <Trans key="columns.thread_name" />
                    </div>
                    <ThreadNameColumn
                      selected={props.columns.threadName}
                      onClick={() =>
                        props.setColumns({
                          ...props.columns,
                          threadName: !props.columns.threadName
                        })
                      }
                    />
                  </div>
                  <div class="w-auto flex flex-col items-center gap-3">
                    <div class="text-sm text-lightSlate-600 text-center">
                      <Trans key="columns.level" />
                    </div>
                    <LogLevelColumn
                      selected={props.columns.level}
                      onClick={() =>
                        props.setColumns({
                          ...props.columns,
                          level: !props.columns.level
                        })
                      }
                    />
                  </div>
                </div>
              </RightHandSide>
            </Row>
            <hr class="h-px w-full bg-darkSlate-400 border-0 rounded" />
            <Row>
              <div>
                <Trans key="logs.autofollow" />
              </div>
              <RightHandSide>
                <div class="flex gap-6 w-full h-auto items-center">
                  <Switch
                    checked={props.autoFollowPreference}
                    onChange={(e) => {
                      props.setAutoFollowPreference(e.currentTarget.checked);
                    }}
                  />
                </div>
              </RightHandSide>
            </Row>
            <hr class="h-px w-full bg-darkSlate-400 border-0 rounded" />
            <Row>
              <div>
                <Trans key="logs.start_log_message_on_new_line" />
              </div>
              <RightHandSide>
                <div class="flex gap-6 w-full h-auto items-center">
                  <Switch
                    checked={props.startLogMessageOnNewLine}
                    onChange={(e) => {
                      props.setStartLogMessageOnNewLine(
                        e.currentTarget.checked
                      );
                    }}
                  />
                </div>
              </RightHandSide>
            </Row>
          </RowsContainer>
        </div>
      )}
    >
      <div
        class="w-6 h-6 hover:bg-lightSlate-50 transition-colors duration-200 ease-in-out i-ri:settings-3-line"
        classList={{
          "bg-lightSlate-800": !isOpen(),
          "bg-lightSlate-50": isOpen()
        }}
      />
    </Popover>
  );
}
