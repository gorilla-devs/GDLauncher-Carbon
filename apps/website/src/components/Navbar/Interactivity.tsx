function Interactivity() {
  if (globalThis.document) {
    const burger = globalThis.document.querySelector(".burger");

    burger.addEventListener("click", () => {
      const classListMenu =
        globalThis.document.querySelector(".mobile-menu").classList;
      classListMenu.toggle("hidden");
      classListMenu.toggle("flex");

      globalThis.document
        .querySelector(".btn-brg")
        .classList.toggle("opened-burger");
    });
  }

  return <></>;
}

export default Interactivity;
