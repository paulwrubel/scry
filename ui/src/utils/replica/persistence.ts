/** Ask the browser not to evict this origin's storage. Denial is not an error. */
export async function requestPersistence(): Promise<boolean> {
  try {
    const storage = globalThis.navigator?.storage;
    if (!storage || typeof storage.persist !== 'function') {
      return false;
    }
    return await storage.persist();
  } catch {
    return false;
  }
}
