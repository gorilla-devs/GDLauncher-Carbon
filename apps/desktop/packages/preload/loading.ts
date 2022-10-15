export function useLoading() {
  const oDiv = document.createElement("div");
  oDiv.style.position = "fixed";
  oDiv.style.top = "0";
  oDiv.style.left = "0";
  oDiv.style.width = "100%";
  oDiv.style.height = "100%";
  oDiv.style.zIndex = "999999";
  oDiv.style.backgroundColor = "#282c34";
  oDiv.style.display = "flex";
  oDiv.style.justifyContent = "center";
  oDiv.style.alignItems = "center";
  oDiv.style.fontSize = "1.5rem";
  
  return {
    appendLoading() {
      oDiv.innerHTML = `<div>Loading...</div>`;
      oDiv.className = "appLoadingState";
      oDiv.style.color = "#fff";
      oDiv.style.fontWeight = "600";
      document.body.appendChild(oDiv);
    },
    clearState() {
      document.body.removeChild(oDiv);
    },
    fatalError(error: Error) {
      oDiv.innerHTML = `<div>${error}</div>`;
      oDiv.className = "appFatalCrashState";
      oDiv.style.color = "#fff";
      oDiv.style.fontWeight = "600";
    },
  };
}
