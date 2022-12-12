import ContentWrapper from "@/layouts/ContentWrapper";
import { useNavigate, useParams } from "@solidjs/router";

const Modpack = () => {
  const navigate = useNavigate();
  const params = useParams();

  return (
    <ContentWrapper>
      <div>Modpack {params.id}</div>
      <button onClick={() => navigate("/modpacks")}>back</button>
    </ContentWrapper>
  );
};

export default Modpack;
