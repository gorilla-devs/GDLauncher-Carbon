import { useNavigate } from "@solidjs/router";

const CodeStep = () => {
  const navigate = useNavigate();

  return (
    <div>
      <button
        onClick={() => {
          navigate("/home");
        }}
      >
        auth
      </button>
    </div>
  );
};

export default CodeStep;
