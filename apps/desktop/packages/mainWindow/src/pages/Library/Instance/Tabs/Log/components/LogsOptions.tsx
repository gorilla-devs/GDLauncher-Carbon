import RightHandSide from "@/pages/Settings/components/RightHandSide";
import Row from "@/pages/Settings/components/Row";
import RowsContainer from "@/pages/Settings/components/RowsContainer";
import { Trans } from "@gd/i18n";
import { Checkbox, Popover, Radio, Slider } from "@gd/ui";
import { createSignal } from "solid-js";

function LowDensityRows() {
  return (
    <div class="w-24 h-10 flex flex-col justify-center items-center box-border border-2 border-solid border-darkSlate-100 rounded-md">
      <div class="w-14 h-3 rounded-sm bg-darkSlate-100" />
    </div>
  );
}

function MediumDensityRows() {
  return (
    <div class="w-24 h-10 flex flex-col gap-1 justify-center items-center box-border border-2 border-solid border-darkSlate-100 rounded-md">
      <div class="w-14 h-1.5 rounded-sm bg-darkSlate-100" />
      <div class="w-14 h-1.5 rounded-sm bg-darkSlate-100" />
    </div>
  );
}

function HighDensityRows() {
  return (
    <div class="w-24 h-10 flex flex-col gap-1 justify-center items-center box-border border-2 border-solid border-darkSlate-100 rounded-md">
      <div class="w-14 h-1 rounded-sm bg-darkSlate-100" />
      <div class="w-14 h-1 rounded-sm bg-darkSlate-100" />
      <div class="w-14 h-1 rounded-sm bg-darkSlate-100" />
      <div class="w-14 h-1 rounded-sm bg-darkSlate-100" />
    </div>
  );
}

function TimestampColumn() {
  return (
    <div class="w-24 h-10 flex gap-1 justify-center items-center box-border border-2 border-solid border-darkSlate-100 rounded-md">
      <div class="w-4 h-2 rounded-sm bg-lightSlate-50" />
      <div class="w-4 h-2 rounded-sm bg-darkSlate-100" />
      <div class="w-10 h-2 rounded-sm bg-darkSlate-100" />
    </div>
  );
}
function LogLevelColumn() {
  return (
    <div class="w-24 h-10 flex gap-1 justify-center items-center box-border border-2 border-solid border-darkSlate-100 rounded-md">
      <div class="w-4 h-2 rounded-sm bg-darkSlate-100" />
      <div class="w-4 h-2 rounded-sm bg-lightSlate-50" />
      <div class="w-10 h-2 rounded-sm bg-darkSlate-100" />
    </div>
  );
}

export type LogDensity = "low" | "medium" | "high";

export type Columns = {
  timestamp: boolean;
  level: boolean;
};

type Props = {
  logsDensity: LogDensity;
  setLogsDensity: (_: LogDensity) => void;
  columns: Columns;
  setColumns: (_: Columns) => void;
  fontMultiplier: 0 | 1 | 2;
  setFontMultiplier: (_: 0 | 1 | 2) => void;
};

export default function LogsOptions(props: Props) {
  const [isOpen, setIsOpen] = createSignal(false);

  return (
    <Popover
      placement="top"
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
                    <LowDensityRows />
                    <Radio
                      value={"low"}
                      checked={props.logsDensity === "low"}
                      onChange={(value) => {
                        props.setLogsDensity(value as LogDensity);
                      }}
                    />
                  </div>
                  <div class="w-full flex flex-col items-center gap-3">
                    <div class="text-sm text-lightSlate-600 text-center">
                      <Trans key="logs_density.comfortable" />
                    </div>
                    <MediumDensityRows />
                    <Radio
                      value={"medium"}
                      checked={props.logsDensity === "medium"}
                      onChange={(value) => {
                        props.setLogsDensity(value as LogDensity);
                      }}
                    />
                  </div>
                  <div class="w-full flex flex-col items-center gap-3">
                    <div class="text-sm text-lightSlate-600 text-center">
                      <Trans key="logs_density.compact" />
                    </div>
                    <HighDensityRows />
                    <Radio
                      value={"high"}
                      checked={props.logsDensity === "high"}
                      onChange={(value) => {
                        props.setLogsDensity(value as LogDensity);
                      }}
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
                <div class="flex gap-6 w-full h-auto items-center">
                  <div class="w-full flex flex-col items-center gap-3">
                    <div class="text-sm text-lightSlate-600 text-center">
                      <Trans key="columns.timestamp" />
                    </div>
                    <TimestampColumn />
                    <Checkbox
                      checked={props.columns.timestamp}
                      onChange={(checked) => {
                        props.setColumns({
                          ...props.columns,
                          timestamp: checked
                        });
                      }}
                    />
                  </div>
                  <div class="w-full flex flex-col items-center gap-3">
                    <div class="text-sm text-lightSlate-600 text-center">
                      <Trans key="columns.level" />
                    </div>
                    <LogLevelColumn />
                    <Checkbox
                      checked={props.columns.level}
                      onChange={(checked) => {
                        props.setColumns({
                          ...props.columns,
                          level: checked
                        });
                      }}
                    />
                  </div>
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
