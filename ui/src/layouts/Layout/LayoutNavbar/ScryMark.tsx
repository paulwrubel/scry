import scryLogo from '../../../../design/scry-logo.svg';

export type ScryMarkProps = {
  className?: string;
};

export function ScryMark(props: ScryMarkProps) {
  return <img src={scryLogo} alt="" aria-hidden="true" className={props.className} />;
}
