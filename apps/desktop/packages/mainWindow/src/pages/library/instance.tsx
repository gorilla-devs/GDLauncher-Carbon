import { useNavigate, useParams } from "@solidjs/router";

const Instace = () => {
  const params = useParams();
  const navigate = useNavigate();

  return (
    <div>
      <div>instace {params.id}</div>
      <button onClick={() => navigate("/library")}>back</button>
    </div>
  );
};

export default Instace;
