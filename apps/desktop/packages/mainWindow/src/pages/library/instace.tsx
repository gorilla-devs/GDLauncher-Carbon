import Page from "@/components/Page";
import { useParams } from "@solidjs/router";

const Instace = () => {
  const params = useParams();

  return (
    <Page class="bg-black-black">
      <div>instace {params.id}</div>
    </Page>
  );
};

export default Instace;
