export const ACTIVE_PROJECT_STORAGE_KEY = 'scry.activeProjectId';

export function readActiveProjectId(): string | null {
  if (typeof localStorage === 'undefined') {
    return null;
  }
  return localStorage.getItem(ACTIVE_PROJECT_STORAGE_KEY);
}

export function writeActiveProjectId(projectId: string | null): void {
  if (typeof localStorage === 'undefined') {
    return;
  }
  if (projectId === null) {
    localStorage.removeItem(ACTIVE_PROJECT_STORAGE_KEY);
    return;
  }
  localStorage.setItem(ACTIVE_PROJECT_STORAGE_KEY, projectId);
}

export function resolveActiveProjectId(
  projectIds: readonly string[],
  preferred: string | null,
): string | null {
  if (preferred !== null && projectIds.includes(preferred)) {
    return preferred;
  }
  return projectIds[0] ?? null;
}
