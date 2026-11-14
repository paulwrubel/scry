import { render } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import type { Operation } from '@/generated/Operation';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import { ReplicaContext } from '@/providers/Replica/context';
import type { ReplicaContextValue } from '@/providers/Replica/context';
import { BoardPage } from '@/views/projects/[projectId]/BoardPage';
import { createReplicaMock, makeReplicaSnapshot } from '@/test/fixtures';
import { SearchProbe } from '@/test/SearchProbe';

export function renderBoard(
  project: ProjectSnapshot,
  enqueue: (operation: Operation) => Promise<void>,
  path = `/projects/${project.id}`,
) {
  const snapshot = makeReplicaSnapshot([project]);

  const context: ReplicaContextValue = {
    status: 'ready',
    replica: createReplicaMock(snapshot, { enqueue }),
    error: undefined,
    retry: () => undefined,
  };

  render(
    <ReplicaContext.Provider value={context}>
      <MemoryRouter initialEntries={[path]}>
        <SearchProbe />
        <Routes>
          <Route path="/projects/:projectId" element={<BoardPage />} />
        </Routes>
      </MemoryRouter>
    </ReplicaContext.Provider>,
  );
}
