// import { createSignal, onMount, onCleanup } from "solid-js";

function TruncatedPath(props: { originalPath: string }) {
  // const [displayPath, setDisplayPath] = createSignal(props.originalPath);
  // let containerRef: HTMLDivElement | undefined = undefined;

  // // Utility to measure text width
  // const measureTextWidth = (text: string, font: string): number => {
  //   const canvas = document.createElement("canvas");
  //   const context = canvas.getContext("2d")!;
  //   context.font = font;
  //   return context.measureText(text).width;
  // };

  // // Adjusted truncation logic
  // const truncatePathMiddle = (path: string, container: HTMLDivElement) => {
  //   const style = window.getComputedStyle(container);
  //   const font = `${style.fontSize} ${style.fontFamily}`;
  //   const maxWidth = container.offsetWidth;
  //   const ellipsis = "...";

  //   if (measureTextWidth(path, font) <= maxWidth) {
  //     return path; // The path fits without needing truncation.
  //   }

  //   let start = 0;
  //   let end = path.length;
  //   let bestFit = "";

  //   // Iteratively find the best fit for the start and end of the path
  //   while (start < end) {
  //     const mid = Math.floor((start + end) / 2);
  //     const part1 = path.substring(0, mid);
  //     const part2 = path.substring(path.length - mid, path.length);
  //     const testPath = `${part1}${ellipsis}${part2}`;

  //     if (measureTextWidth(testPath, font) <= maxWidth) {
  //       bestFit = testPath;
  //       start = mid + 1; // Try to show more of the path
  //     } else {
  //       end = mid - 1;
  //     }
  //   }

  //   return bestFit || ellipsis; // Return the best fit found or ellipsis if nothing fits
  // };

  // // Initialize ResizeObserver to adjust truncation based on container size
  // const setupResizeObserver = () => {
  //   const observer = new ResizeObserver(() => {
  //     if (containerRef) {
  //       const newPath = truncatePathMiddle(props.originalPath, containerRef);
  //       setDisplayPath(newPath);
  //     }
  //   });

  //   if (containerRef) {
  //     observer.observe(containerRef);
  //   }

  //   onCleanup(() => observer.disconnect());
  // };

  // onMount(() => {
  //   if (containerRef) {
  //     const newPath = truncatePathMiddle(props.originalPath, containerRef);
  //     setDisplayPath(newPath);
  //     setupResizeObserver();
  //   }
  // });

  return (
    // <div ref={containerRef} class="w-full overflow-hidden whitespace-nowrap">
    //   {displayPath()}
    // </div>
    <div class="w-full whitespace-nowrap break-all overflow-hidden truncate">
      {props.originalPath}
    </div>
  );
}

export default TruncatedPath;
