import { useNavigate, useParams } from "@solidjs/router";

const Modpack = () => {
  const navigate = useNavigate();
  const params = useParams();

  return (
    <div>
      <div>Modpack {params.id}</div>
      <button onClick={() => navigate("/modpacks")}>back</button>
    </div>
  );
};

export default Modpack;
