import { useModal } from "@/managers/ModalsManager";
import { useTransContext } from "@gd/i18n";
import { Button } from "@gd/ui";

const BeginExport = () => {
  const [t] = useTransContext();
  const modalsContext = useModal();
  return (
    <div class="flex justify-between items-center w-full pt-4">
      <Button
        type="secondary"
        size="large"
        onClick={() => {
          modalsContext?.closeModal();
        }}
      >
        {t("instance.cancel_export")}
      </Button>
      <Button type="primary" size="large">
        {t("instance.begin_export")}
      </Button>
    </div>
  );
};
export default BeginExport;
