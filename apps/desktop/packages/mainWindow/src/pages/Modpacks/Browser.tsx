import { useNavigate } from "@solidjs/router";

export default function About() {
  const navigate = useNavigate();

  return (
    <div>
      <div>
        <button onClick={() => navigate("/modpacks/DDLTR")}>
          Modpack DDLTR
        </button>
      </div>
    </div>
  );
}
