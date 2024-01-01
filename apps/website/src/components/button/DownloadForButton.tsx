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
  const navigateToDownload = () => {
    const userAgent = navigator.userAgent.split(" ")[1].split(" ")[0].slice(1);
    if (userAgent.toLowerCase().includes("windows")) {
      window.location.href =
        "https://cdn-raw.gdl.gg/launcher/GDLauncher__2.0.0-alpha.1703815106__win__x64.exe";
    } else if (userAgent.toLowerCase().includes("linux")) {
      window.location.href =
        "https://cdn-raw.gdl.gg/launcher/GDLauncher__2.0.0-alpha.1703815106__linux__x64.AppImage";
    } else {
      window.location.href =
        "https://cdn-raw.gdl.gg/launcher/GDLauncher__2.0.0-alpha.1703815106__mac__universal.dmg";
    }
  };

  return (
    <Button
      onClick={navigateToDownload}
      intent="primary"
      class="uppercase items-center"
    >
      <span>DOWNLOAD FOR</span>
      {getOs()}
    </Button>
  );
};
