import { onMount } from "solid-js";

const ArticleInteractiveMarkdown = () => {
  onMount(() => {
    const container = document.querySelector("#articleContainer");
    // make table of content dynamic based on h1 titles
    container?.querySelectorAll("h1").forEach((item) => {
        item.style.cursor = "pointer";
        item.style.display = "inline";
        const url = new URL(window.location.toString());
        // Make sure to prevent multiple ids being added to the URL
        // Add hover effect on title, then make an icon appear
        // on the left and make that clickable like github
        item.onclick = () => {
            const id = item.id;
            history.pushState({}, "", url + "#" + id);
        }
    })
  });
  return <></>;
};

export default ArticleInteractiveMarkdown;
