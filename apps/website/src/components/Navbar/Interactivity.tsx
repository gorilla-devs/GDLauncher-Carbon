function Interactivity() {
  if (globalThis.document) {
    const burger = globalThis.document.querySelector(".burger");

    (burger || ({} as Element)).addEventListener("click", () => {
      const mobileMenu = globalThis.document.querySelector(".mobile-menu");

      const mobileMenuDisplayStyle = (mobileMenu || ({} as any)).style.display;

      if (mobileMenuDisplayStyle) {
        (mobileMenu || ({} as any)).style.display =
          mobileMenuDisplayStyle === "none" ? "flex" : "none";
      } else (mobileMenu || ({} as any)).style.display = "flex";

      const btnBrg = globalThis.document.querySelector(".btn-brg");

      ((btnBrg || {}) as Element).classList.toggle("opened-burger");
    });
  }

  return <></>;
}

export default Interactivity;
