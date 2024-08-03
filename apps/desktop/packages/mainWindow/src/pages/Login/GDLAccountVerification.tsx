import { useGDNavigate } from "@/managers/NavigationManager";
import { rspc } from "@/utils/rspcClient";
import { Input } from "@gd/ui";
import { onMount } from "solid-js";

interface Props {
  nextStep: () => void;
  prevStep: () => void;
}

const GDLAccountVerification = (props: Props) => {
  const rspcContext = rspc.useContext();
  const navigate = useGDNavigate();

  onMount(async () => {
    const activeUuid = await rspcContext.client.query([
      "account.getActiveUuid"
    ]);

    if (!activeUuid) {
      throw new Error("No active uuid");
    }

    const waitingForVerification = await rspcContext.client.query([
      "account.waitForGdlAccountVerification",
      activeUuid
    ]);

    if (waitingForVerification) {
      navigate("/library");
    }
  });

  return (
    <div class="flex-1 w-full flex flex-col justify-between items-center text-center gap-5 p-10">
      <h1>Waiting for verification</h1>
    </div>
  );
};

export default GDLAccountVerification;
