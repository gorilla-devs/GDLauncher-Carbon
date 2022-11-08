import { useNavigate } from "@solidjs/router";
import Logo from "../../assets/images/gdlauncher_vertical_logo.svg";

export default function Home() {
  const navigate = useNavigate();

  return (
    <div class="flex justify-center items-center w-full h-full bg-image-loginBG p-0">
      <div
        style={{
          "mix-blend-mode": "hard-light",
        }}
        class="absolute top-0 left-0 right-0 bottom-0 bg-[#1D2028] opacity-80"
      />
      <div class="w-120 h-90 rounded-2xl bg-[#1D2028] opacity-80 relative backdrop-blur-sm flex flex-col justify-end items-center text-white">
        <div class="absolute left-0 right-0 m-auto -top-15 flex flex-col justify-center items-center">
          <img class="w-40" src={Logo} />
          <p class="text-[#8A8B8F]">v1.1.26</p>
        </div>
        <div class="text-center flex flex-col justify-center items-center">
          <button
            onClick={() => {
              navigate("/home");
            }}
          >
            Auth
          </button>
          <p class="max-w-90 text-sm text-[#8A8B8F]">
            Sign in with your Microsoft Account. By doing so, you accept all our
            policies and terms stated below.
          </p>
          <ul class="flex gap-3 list-none p-0 mb-8">
            <li>Privacy Policy</li>
            <li>Terms and Conditions</li>
            <li>Acceptable Use Policy</li>
          </ul>
        </div>
      </div>
    </div>
  );
}
