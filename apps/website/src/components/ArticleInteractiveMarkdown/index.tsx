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
  onMount(async () => {
    const container = document.querySelector("#articleContainer");
    const tableListTitle = document.querySelector("#tableListSummary");
    const tableList = document.getElementById("tableList");
    tableList!.style.marginLeft = "10px";
    if (tableListTitle) (tableListTitle as any).style.cursor = "pointer";

    container?.querySelectorAll("h1").forEach((item) => {
      item.style.maxWidth = "fit-content";
      item.style.display = "flex";
      item.style.flexDirection = "row-reverse";
      item.style.position = "relative";
      item.style.gap = "1rem";
      const iconSvg = document.createElementNS(
        "http://www.w3.org/2000/svg",
        "svg",
      );

      const iconPath = document.createElementNS(
        "http://www.w3.org/2000/svg",
        "path",
      );

      iconSvg.setAttribute("viewBox", "0 0 640 512");
      iconSvg.setAttribute("width", "20");
      iconPath.setAttribute(
        "d",
        "M579.8 267.7c56.5-56.5 56.5-148 0-204.5c-50-50-128.8-56.5-186.3-15.4l-1.6 1.1c-14.4 10.3-17.7 30.3-7.4 44.6s30.3 17.7 44.6 7.4l1.6-1.1c32.1-22.9 76-19.3 103.8 8.6c31.5 31.5 31.5 82.5 0 114L422.3 334.8c-31.5 31.5-82.5 31.5-114 0c-27.9-27.9-31.5-71.8-8.6-103.8l1.1-1.6c10.3-14.4 6.9-34.4-7.4-44.6s-34.4-6.9-44.6 7.4l-1.1 1.6C206.5 251.2 213 330 263 380c56.5 56.5 148 56.5 204.5 0L579.8 267.7zM60.2 244.3c-56.5 56.5-56.5 148 0 204.5c50 50 128.8 56.5 186.3 15.4l1.6-1.1c14.4-10.3 17.7-30.3 7.4-44.6s-30.3-17.7-44.6-7.4l-1.6 1.1c-32.1 22.9-76 19.3-103.8-8.6C74 372 74 321 105.5 289.5L217.7 177.2c31.5-31.5 82.5-31.5 114 0c27.9 27.9 31.5 71.8 8.6 103.9l-1.1 1.6c-10.3 14.4-6.9 34.4 7.4 44.6s34.4 6.9 44.6-7.4l1.1-1.6C433.5 260.8 427 182 377 132c-56.5-56.5-148-56.5-204.5 0L60.2 244.3z",
      );
      iconSvg.setAttribute("fill", "#e2e8f0");
      iconSvg.appendChild(iconPath);

      const div = document.createElement("div");
      div.appendChild(iconSvg);

      div.style.cursor = "pointer";
      div.style.opacity = "0";
      div.style.width = "40px";
      div.style.height = "100%";
      div.style.position = "absolute";
      div.style.left = "-2.5rem";
      div.style.display = "flex";
      div.style.justifyContent = "center";
      div.style.alignItems = "center";

      item.onmouseenter = () => {
        div.style.opacity = "1";
      };
      item.onmouseleave = () => {
        div.style.opacity = "0";
      };

      const li = document.createElement("li");
      const a = document.createElement("a");
      const textnode = document.createTextNode(item.innerHTML);
      a.appendChild(textnode);
      a.style.fontSize = "1rem";
      a.style.maxWidth = "fit-content";
      a.style.cursor = "pointer";
      a.style.color = "#60a5fa";

      a.onclick = () => {
        const itemId = item.id;
        history.pushState({}, "", "#" + itemId);

        item.scrollIntoView({ behavior: "smooth" });
      };

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
          history.pushState({}, "", "#" + subItemId);

          subItem.scrollIntoView({ behavior: "smooth" });
        };

        ul.appendChild(subLi);

        subLi.style.cursor = "pointer";
        subLi.style.maxWidth = "fit-content";
        subLi.style.marginLeft = "10px";
        subLi.style.listStyleType = "circle";
        subItem.style.position = "relative";
        anchorTag.style.color = "#60a5fa";
        li.appendChild(ul);

        subItem.onclick = () => {
          const subItemId = subItem.id;
          history.pushState({}, "", "#" + subItemId);
        };

        const iconSvg = document.createElementNS(
          "http://www.w3.org/2000/svg",
          "svg",
        );

        const iconPath = document.createElementNS(
          "http://www.w3.org/2000/svg",
          "path",
        );

        iconSvg.setAttribute("viewBox", "0 0 640 512");
        iconSvg.setAttribute("width", "20");
        iconPath.setAttribute(
          "d",
          "M579.8 267.7c56.5-56.5 56.5-148 0-204.5c-50-50-128.8-56.5-186.3-15.4l-1.6 1.1c-14.4 10.3-17.7 30.3-7.4 44.6s30.3 17.7 44.6 7.4l1.6-1.1c32.1-22.9 76-19.3 103.8 8.6c31.5 31.5 31.5 82.5 0 114L422.3 334.8c-31.5 31.5-82.5 31.5-114 0c-27.9-27.9-31.5-71.8-8.6-103.8l1.1-1.6c10.3-14.4 6.9-34.4-7.4-44.6s-34.4-6.9-44.6 7.4l-1.1 1.6C206.5 251.2 213 330 263 380c56.5 56.5 148 56.5 204.5 0L579.8 267.7zM60.2 244.3c-56.5 56.5-56.5 148 0 204.5c50 50 128.8 56.5 186.3 15.4l1.6-1.1c14.4-10.3 17.7-30.3 7.4-44.6s-30.3-17.7-44.6-7.4l-1.6 1.1c-32.1 22.9-76 19.3-103.8-8.6C74 372 74 321 105.5 289.5L217.7 177.2c31.5-31.5 82.5-31.5 114 0c27.9 27.9 31.5 71.8 8.6 103.9l-1.1 1.6c-10.3 14.4-6.9 34.4 7.4 44.6s34.4 6.9 44.6-7.4l1.1-1.6C433.5 260.8 427 182 377 132c-56.5-56.5-148-56.5-204.5 0L60.2 244.3z",
        );
        iconSvg.setAttribute("fill", "#e2e8f0");
        iconSvg.appendChild(iconPath);

        const subDiv = document.createElement("div");
        subDiv.appendChild(iconSvg);

        subDiv.style.cursor = "pointer";
        subDiv.style.opacity = "0";
        subDiv.style.width = "40px";
        subDiv.style.height = "30px";
        subDiv.style.position = "absolute";
        subDiv.style.left = "-2.5rem";
        subDiv.style.top = "0";
        subDiv.style.display = "flex";
        subDiv.style.justifyContent = "center";
        subDiv.style.alignItems = "center";

        subItem.onmouseenter = () => {
          subDiv.style.opacity = "1";
        };
        subItem.onmouseleave = () => {
          subDiv.style.opacity = "0";
        };

        subItem.append(subDiv);
      });

      if (li.childNodes.length === 1) {
        li.style.cursor = "pointer";

        li.onclick = () => {
          const id = item.id;
          history.pushState({}, "", "#" + id);
          item.scrollIntoView({ behavior: "smooth" });
        };
      }

      tableList?.appendChild(li);

      item.append(div);

      // Make sure to prevent multiple ids being added to the URL
      // Add hover effect on title, then make an icon appear
      // on the left and make that clickable like github
      iconSvg.onclick = () => {
        const id = item.id;
        history.pushState({}, "", "#" + id);
      };

      container?.querySelectorAll("img").forEach((item) => {
        item.style.maxWidth = "100%";
      });
    });
  });
  return <></>;
};

export default ArticleInteractiveMarkdown;
