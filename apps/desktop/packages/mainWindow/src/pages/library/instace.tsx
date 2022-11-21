import Page from "@/components/Page";
import { useNavigate, useParams } from "@solidjs/router";

const Instace = () => {
  const params = useParams();
  const navigate = useNavigate();

  return (
    <Page class="bg-black-black">
      <div>instace {params.id}</div>
      <button onClick={() => navigate("/library")}>back</button>
    </Page>
  );
};

export default Instace;
