function Interactivity() {
  if (globalThis.document) {
    const burger = globalThis.document.querySelector(".burger");
    burger.addEventListener("click", () => {
      globalThis.document.querySelector(".navbar").classList.toggle("open");

      globalThis.document
        .querySelector(".btn-brg")
        .classList.toggle("opened-burger");
    });
  }

  return <></>;
}

export default Interactivity;
