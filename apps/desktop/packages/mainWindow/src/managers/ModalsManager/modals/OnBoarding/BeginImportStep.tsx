import { For, createEffect } from "solid-js";
import SingleImport from "./SingleImport";

const BeginImportStep = (props: {
  singleInstance?: string;
  instances?: Array<string>;
}) => {
  createEffect(() => {
    console.log(props.instances);
  });
  return (
    <div class=" w-full h-full p-2">
      <For each={props.instances}>
        {(instance, index) => (
          <SingleImport instanceIndex={index()} instanceName={instance} />
        )}
      </For>
    </div>
  );
};
export default BeginImportStep;
