import { useSearchParams } from 'react-router-dom';

export function SearchProbe() {
  const [params] = useSearchParams();
  return <span data-testid="search">{params.toString()}</span>;
}
