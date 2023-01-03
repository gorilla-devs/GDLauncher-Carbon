import { useNavigate, useParams } from "@solidjs/router";
import headerMockImage from "/assets/images/minecraft-forge.jpg";

const Instace = () => {
  const params = useParams();
  const navigate = useNavigate();

  return (
    <>
      <div
        class="relative h-full bg-fixed bg-no-repeat max-h-full overflow-auto"
        style={{
          "background-image": `url("${headerMockImage}")`,
          "background-position": "center -5rem",
        }}
      >
        <div class="mt-64 h-200 bg-black-black">
          <div class="mt-65">instace {params.id}</div>
          <button onClick={() => navigate("/library")}>back</button>
        </div>
      </div>
    </>
  );
};

export default Instace;
