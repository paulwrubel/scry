import { useReplicaStatus } from './hook';

export type ReplicaGateProps = {
  children: React.ReactNode;
  /** Rendered until the replica is ready. */
  fallback?: React.ReactNode;
};

/** Mounts its children only once the replica is open, so providers that read the replica can assume it exists. */
export function ReplicaGate(props: ReplicaGateProps) {
  const { children, fallback = null } = props;

  const { status } = useReplicaStatus();

  return status === 'ready' ? <>{children}</> : <>{fallback}</>;
}
