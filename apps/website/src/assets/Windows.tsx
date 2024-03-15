function Windows(props: { fill?: string; width?: number; height?: number }) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width={props.width || "20"}
      height={props.height || "20"}
      fill="none"
      viewBox="0 0 20 20"
    >
      <path
        fill={props.fill || "#fff"}
        d="M2.5 4.566l6.147-.847v5.94H2.5V4.565zm0 10.868l6.147.847v-5.866H2.5v5.02zm6.824.938L17.5 17.5v-7.085H9.324v5.957zm0-12.744v6.03H17.5V2.5L9.324 3.628z"
      ></path>
    </svg>
  );
}

export default Windows;
