function storageKey(projectId: string): string {
  return `scry.collapsedSections.${projectId}`;
}

export function readCollapsedSections(projectId: string): ReadonlySet<string> {
  if (typeof localStorage === 'undefined') {
    return new Set();
  }

  const raw = localStorage.getItem(storageKey(projectId));
  if (raw === null) {
    return new Set();
  }

  try {
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) {
      return new Set();
    }
    return new Set(parsed.filter((entry): entry is string => typeof entry === 'string'));
  } catch {
    return new Set();
  }
}

export function writeCollapsedSections(projectId: string, ids: Iterable<string>): void {
  if (typeof localStorage === 'undefined') {
    return;
  }

  const values = [...ids];
  if (values.length === 0) {
    localStorage.removeItem(storageKey(projectId));
    return;
  }

  localStorage.setItem(storageKey(projectId), JSON.stringify(values));
}
