import Page from "@/components/Page";
import { useNavigate, useParams } from "@solidjs/router";

const Modpack = () => {
  const navigate = useNavigate();
  const params = useParams();

  return (
    <Page class="bg-black-black">
      <div>Modpack {params.id}</div>
      <button onClick={() => navigate("/modpacks")}>back</button>
    </Page>
  );
};

export default Modpack;
