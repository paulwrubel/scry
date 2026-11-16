// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import { afterEach, describe, expect, it } from 'vitest';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import { ReplicaContext } from '@/providers/Replica/context';
import type { ReplicaContextValue } from '@/providers/Replica/context';
import { createReplicaMock, makeEvent, makeProject, makeReplicaSnapshot } from '@/test/fixtures';
import { ProjectEventsLayout } from './ProjectEventsLayout';

const PROJECT_ID = 'project-1';

function projectSnapshot(): ProjectSnapshot {
  return makeProject({ events: [makeEvent({ id: 'event-1', title: 'Standup' })] });
}

function LocationProbe() {
  const location = useLocation();
  return (
    <span data-testid="path">
      {location.pathname}
      {location.search}
    </span>
  );
}

function renderLayout(path: string) {
  const snapshot = makeReplicaSnapshot([projectSnapshot()]);
  const context: ReplicaContextValue = {
    status: 'ready',
    replica: createReplicaMock(snapshot),
    error: undefined,
    retry: () => undefined,
  };

  render(
    <ReplicaContext.Provider value={context}>
      <MemoryRouter initialEntries={[path]}>
        <LocationProbe />
        <Routes>
          <Route path="/projects/:projectId/events" element={<ProjectEventsLayout />}>
            <Route path="days/:date" />
            <Route path=":eventId" />
          </Route>
        </Routes>
      </MemoryRouter>
    </ReplicaContext.Provider>,
  );
}

afterEach(cleanup);

describe('ProjectEventsLayout', () => {
  it('opens the day list for a date', () => {
    renderLayout(`/projects/${PROJECT_ID}/events/days/2026-11-06?month=2026-11`);

    const drawer = within(screen.getByTestId('flowbite-drawer'));
    expect(drawer.getByRole('heading', { name: 'Friday' })).toBeDefined();
    expect(drawer.getByText('Standup')).toBeDefined();
    expect(screen.getByTestId('flowbite-drawer').getAttribute('aria-hidden')).toBe('false');
  });

  it('opens the editor for an event id', () => {
    renderLayout(`/projects/${PROJECT_ID}/events/event-1?month=2026-11`);

    expect(screen.getByRole('button', { name: 'Save changes' })).toBeDefined();
  });

  it('closes the day list back to the calendar at the same month', () => {
    renderLayout(`/projects/${PROJECT_ID}/events/days/2026-11-06?month=2026-11`);

    fireEvent.click(screen.getByRole('button', { name: 'Back to calendar' }));

    expect(screen.getByTestId('path').textContent).toBe(
      `/projects/${PROJECT_ID}/events?month=2026-11`,
    );
  });
});
