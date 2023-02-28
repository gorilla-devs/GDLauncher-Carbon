import { useGDNavigate } from "@/managers/NavigationManager";

export default function About() {
  const navigate = useGDNavigate();

  return (
    <div>
      <div>
        <button onClick={() => navigate("/modpacks/DDLTR")}>
          {/* Modpack DDLTR */}
        </button>
      </div>
    </div>
  );
}
