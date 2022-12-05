interface Props {
  class?: string;
}

export const BackgroundPattern = (props: Props) => {
  return <div class={`bg-image-gdlauncher_pattern.svg -z-10 ${props.class}`} />;
};

export default BackgroundPattern;
