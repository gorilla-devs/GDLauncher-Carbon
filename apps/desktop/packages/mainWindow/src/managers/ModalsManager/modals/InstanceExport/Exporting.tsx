import { useTransContext } from "@gd/i18n";
import LoadingGif from "/assets/images/image.gif";
import { Progressbar } from "@gd/ui";
export default function Exporting(props: { progress: number }) {
  const [t] = useTransContext();
  return (
    <div class="w-full gap-4 h-full flex flex-col items-center justify-center">
      <img src={LoadingGif} class="h-40 w-40" alt="loading" />
      <span>{t("instance.exporting_instance")}</span>
      <Progressbar color="bg-primary-500" percentage={props.progress} />
      <span>{`${props.progress}% ${t("instance.export_completed")}`}</span>
    </div>
  );
}
