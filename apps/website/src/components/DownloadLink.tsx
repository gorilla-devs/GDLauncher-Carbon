import { Show, createResource } from "solid-js";
import Apple from "../assets/Apple";
import Linux from "../assets/Linux";
import Windows from "../assets/Windows";

const getOs = () => {
  if (window.navigator.userAgent.toLowerCase().includes("windows")) {
    return "Windows";
  } else if (window.navigator.userAgent.toLowerCase().includes("mac")) {
    return "MacOS";
  } else if (window.navigator.userAgent.toLowerCase().includes("linux")) {
    return "Linux";
  } else {
    return "Unknown";
  }
};

export const DownloadLink = ({ urls }: { urls: Array<string> }) => {
  const getCurrentUrl = () => {
    if (getOs() === "Windows") {
      return urls[0];
    } else if (getOs() === "MacOS") {
      return urls[1];
    } else {
      return urls[2];
    }
  };
  const url = getCurrentUrl();
  return (
    <a href={url} class="flex items-center gap-2">
      <span>DOWNLOAD FOR</span>

      <Show when={getOs() === "Windows"}>
        <Windows />
      </Show>
      <Show when={getOs() === "MacOS"}>
        <Apple />
      </Show>
      <Show when={getOs() === "Linux"}>
        <Linux />
      </Show>
    </a>
  );
};
