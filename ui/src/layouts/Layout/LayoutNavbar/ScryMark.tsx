import scryLogo from '../../../../design/scry-logo.svg';

export type ScryMarkProps = {
  className?: string;
};

export function ScryMark(props: ScryMarkProps) {
  const { className } = props;

  return <img src={scryLogo} alt="" aria-hidden="true" className={className} />;
}
