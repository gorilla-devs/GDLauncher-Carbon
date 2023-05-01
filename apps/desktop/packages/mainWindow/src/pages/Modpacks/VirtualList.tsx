import { For, createSignal, JSX, onMount, onCleanup } from "solid-js";

interface Props<T> {
  items: T[];
  fetchMore?: (_page: number) => void;
  pageSize: number;
  item: (_item: T) => JSX.Element;
  scrollTargetRef: JSX.Element | undefined;
}

const throttle = (callback: any, time: number) => {
  let throttleWait;

  if (throttleWait) return;
  throttleWait = true;

  setTimeout(() => {
    callback();
    throttleWait = false;
  }, time);
};

function VirtualList<T>(props: Props<T>) {
  const [page, setPage] = createSignal(1);
  let containerRef: HTMLDivElement;

  function handleIntersection() {
    setPage(page() + 1);
  }

  const lastIndex = () => page() * props.pageSize;

  const handleScroll = () => {
    const endOfPage =
      Math.round(containerRef.scrollTop + containerRef.clientHeight) >=
      containerRef.scrollHeight;

    if (endOfPage) {
      props.fetchMore && props.fetchMore(setPage((prev) => prev + 1));
      // pageEnd.innerHTML = "true";
      console.log("endOfPage", endOfPage);
      // pageEnd.style.color = "green";
    } else {
      console.log("NOTEND", endOfPage);
      // pageEnd.innerHTML = "false";
      // pageEnd.style.color = "red";
    }
  };

  const throttledScroll = () => {
    throttle(handleScroll, 250);
  };

  onMount(() => {
    containerRef.addEventListener("scroll", throttledScroll);
  });

  onCleanup(() => {
    containerRef.removeEventListener("scroll", throttledScroll);
  });

  return (
    <div
      class="h-full overflow-y-auto flex flex-col gap-2"
      ref={(el) => {
        containerRef = el;
      }}
    >
      <For each={props.items}>{(itemData) => props.item(itemData)}</For>
      <div>Loading...</div>
    </div>
  );
}

export default VirtualList;
