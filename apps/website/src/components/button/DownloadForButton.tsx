import Button from "./Button";
import Windows from "../../assets/Windows";
import Linux from "../../assets/Linux";
import Apple from "../../assets/Apple";

export const DownloadForButton = () => {
  //   here goes the download logic

  const getOs = () => {
    const userAgent = navigator.userAgent.split(" ")[1].split(" ")[0].slice(1);
    if (userAgent.toLowerCase().includes("windows")) return <Windows />;
    if (userAgent.toLowerCase().includes("linux")) return <Linux />;
    return <Apple />;
  };
  return (
    <Button intent="primary" class="uppercase">
      <span>DOWNLOAD FOR</span>
      {getOs()}
    </Button>
  );
};
