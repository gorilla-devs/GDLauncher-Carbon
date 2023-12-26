import { createSignal } from "solid-js";
import Button from "./Button";

export const DownloadForButton = () => {
  //   here goes the download logic

  return (
    <Button intent="primary" className="uppercase">
      {`DOWNLOAD FOR ${navigator.userAgent
        .split(" ")[1]
        .split(" ")[0]
        .slice(1)}`}
    </Button>
  );
};
