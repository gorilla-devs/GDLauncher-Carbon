import { useNavigate } from "@solidjs/router";
import Logo from "../../assets/images/gdlauncher_vertical_logo.svg";

export default function Home() {
  const navigate = useNavigate();

  return (
    <div class="flex justify-center items-center w-full h-full bg-image-loginBG p-0">
      <div class="w-120 h-90 rounded-2xl bg-[#1D2028] opacity-80 relative backdrop-blur-sm flex flex-col justify-center items-center">
        <img class="w-40 absolute left-0 right-0 m-auto -top-15" src={Logo} />
        <div class="max-w-90 text-white text-center">
          <button
            onClick={() => {
              navigate("/home");
            }}
          >
            Auth
          </button>
          <p class="text-sm">
            Sign in with your Microsoft Account. By doing so, you accept all our
            policies and terms stated below.
          </p>
        </div>
      </div>
    </div>
  );
}
