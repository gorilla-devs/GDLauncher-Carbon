type Props = {
  class?: string;
};

export const Pattern = (props: Props) => {
  return <div class={`bg-pattern-gdlauncher_pattern.svg -z-10 ${props.class}`} />;
};

export default Pattern;
