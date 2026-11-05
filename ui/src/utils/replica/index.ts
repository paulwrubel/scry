export { openReplica } from './replica';
export type { Replica } from './replica';
export { requestPersistence } from './persistence';
export type { LoadedConfirmed } from './storage';
export {
  CONFIRMED_SCHEMA_VERSION,
  REPLICA_DATABASE,
  REPLICA_DATABASE_VERSION,
  isReplicaError,
  replicaError,
} from './types';
export type { ReplicaError, ReplicaErrorKind, ReplicaOptions, ReplicaSnapshot } from './types';
