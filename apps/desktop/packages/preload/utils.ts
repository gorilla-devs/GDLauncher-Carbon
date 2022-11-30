/** Document ready */
export const domReady = (
  // eslint-disable-next-line no-undef
  condition: DocumentReadyState[] = ["complete", "interactive"]
) => {
  return new Promise((resolve) => {
    if (condition.includes(document.readyState)) {
      resolve(true);
    } else {
      document.addEventListener("readystatechange", () => {
        if (condition.includes(document.readyState)) {
          resolve(true);
        }
      });
    }
  });
};
