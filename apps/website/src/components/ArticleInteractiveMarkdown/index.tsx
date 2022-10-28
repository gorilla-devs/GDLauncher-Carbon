import { onMount } from "solid-js";

const getH2s = (item: HTMLHeadingElement) => {
  let currentSibling: HTMLHeadingElement = item;
  let siblings = [];
  let ok = true;

  while (ok) {
    if (
      (currentSibling?.tagName === "H1" && currentSibling?.id !== item.id) ||
      currentSibling === null
    ) {
      ok = false;
    } else {
      if (currentSibling?.tagName === "H2") {
        siblings.push(currentSibling);
      }
      currentSibling = currentSibling?.nextElementSibling as HTMLHeadingElement;
    }
  }

  return siblings;
};

const ArticleInteractiveMarkdown = () => {
  onMount(() => {
    const container = document.querySelector("#articleContainer");
    const tableList = document.getElementById("tableList");

    container?.querySelectorAll("h1").forEach((item) => {
      item.style.cursor = "pointer";
      item.style.maxWidth = "fit-content";
      item.style.display = "flex";
      item.style.flexDirection = "row-reverse";
      item.style.position = "relative";
      item.style.gap = "1rem";
      const iconSvg = document.createElementNS(
        "http://www.w3.org/2000/svg",
        "svg"
      );
      const iconPath = document.createElementNS(
        "http://www.w3.org/2000/svg",
        "path"
      );

      iconSvg.setAttribute("viewBox", "0 0 640 512");
      iconSvg.setAttribute("width", "20");
      iconPath.setAttribute(
        "d",
        "M579.8 267.7c56.5-56.5 56.5-148 0-204.5c-50-50-128.8-56.5-186.3-15.4l-1.6 1.1c-14.4 10.3-17.7 30.3-7.4 44.6s30.3 17.7 44.6 7.4l1.6-1.1c32.1-22.9 76-19.3 103.8 8.6c31.5 31.5 31.5 82.5 0 114L422.3 334.8c-31.5 31.5-82.5 31.5-114 0c-27.9-27.9-31.5-71.8-8.6-103.8l1.1-1.6c10.3-14.4 6.9-34.4-7.4-44.6s-34.4-6.9-44.6 7.4l-1.1 1.6C206.5 251.2 213 330 263 380c56.5 56.5 148 56.5 204.5 0L579.8 267.7zM60.2 244.3c-56.5 56.5-56.5 148 0 204.5c50 50 128.8 56.5 186.3 15.4l1.6-1.1c14.4-10.3 17.7-30.3 7.4-44.6s-30.3-17.7-44.6-7.4l-1.6 1.1c-32.1 22.9-76 19.3-103.8-8.6C74 372 74 321 105.5 289.5L217.7 177.2c31.5-31.5 82.5-31.5 114 0c27.9 27.9 31.5 71.8 8.6 103.9l-1.1 1.6c-10.3 14.4-6.9 34.4 7.4 44.6s34.4 6.9 44.6-7.4l1.1-1.6C433.5 260.8 427 182 377 132c-56.5-56.5-148-56.5-204.5 0L60.2 244.3z"
      );
      iconSvg.setAttribute("fill", "#e2e8f0");
      iconSvg.setAttribute("style", "position: absolute;left: -2rem;");
      iconSvg.setAttribute("opacity", "0");

      item.onmouseenter = () => {
        iconSvg.setAttribute("opacity", "1");
      };
      item.onmouseleave = () => {
        iconSvg.setAttribute("opacity", "0");
      };
      iconSvg.appendChild(iconPath);

      const url = new URL(window.location.toString());

      const li = document.createElement("li");
      const a = document.createElement("a");
      const textnode = document.createTextNode(item.innerHTML);
      a.appendChild(textnode);
      a.style.fontSize = "1rem";
      a.style.maxWidth = "fit-content";

      li.appendChild(a);

      const siblings = getH2s(item);

      siblings.forEach((subItem) => {
        const ul = document.createElement("ul");
        const subLi = document.createElement("li");
        const anchorTag = document.createElement("a");
        const textnode = document.createTextNode(subItem.innerHTML);
        anchorTag.appendChild(textnode);
        anchorTag.style.fontSize = "1rem";

        subLi.appendChild(anchorTag);
        subLi.onclick = () => {
          const subItemId = subItem.id;
          history.pushState({}, "", url + "#" + subItemId);
        };

        ul.appendChild(subLi);

        subLi.style.cursor = "pointer";
        subLi.style.maxWidth = "fit-content";
        li.appendChild(ul);
      });

      if (li.childNodes.length === 1) {
        li.style.cursor = "pointer";

        li.onclick = () => {
          const id = item.id;
          history.pushState({}, "", url + "#" + id);
        };
      }

      tableList?.appendChild(li);

      item.append(iconSvg);

      // Make sure to prevent multiple ids being added to the URL
      // Add hover effect on title, then make an icon appear
      // on the left and make that clickable like github
      item.onclick = () => {
        const id = item.id;
        history.pushState({}, "", url + "#" + id);
      };

      container?.querySelectorAll("img").forEach((item) => {
        item.style.maxWidth = "100%";
      });
    });
  });
  return <></>;
};

export default ArticleInteractiveMarkdown;
