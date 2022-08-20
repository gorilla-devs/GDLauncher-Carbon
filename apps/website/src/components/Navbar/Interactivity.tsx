function Interactivity() {
  if (globalThis.document) {
    const burger = globalThis.document.querySelector(".burger");
    burger.addEventListener("click", () => {
      const classListMenu =
        globalThis.document.querySelector(".mobile-menu").classList;
      classListMenu.toggle("hidden");
      classListMenu.toggle("flex");

      const classListNavbar =
        globalThis.document.querySelector(".navbar").classList;
      classListNavbar.toggle("absolute");
      classListNavbar.toggle("fixed");

      globalThis.document
        .querySelector(".btn-brg")
        .classList.toggle("opened-burger");
    });
  }

  return <></>;
}

export default Interactivity;
