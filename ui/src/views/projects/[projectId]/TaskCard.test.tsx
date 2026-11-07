// @vitest-environment jsdom
import { cleanup, render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, describe, expect, it } from 'vitest';
import type { Task } from '@/generated/Task';
import { TaskCard } from './TaskCard';

const CREATED_AT = '2026-01-01T00:00:00Z';

const task: Task = {
  id: 'task-1',
  project_id: 'project-1',
  title: 'Task title',
  description: null,
  priority: 'medium',
  section_id: null,
  position: 'a0',
  state: 'open',
  tags: [],
  created_at: CREATED_AT,
};

afterEach(cleanup);

describe('TaskCard', () => {
  it('carries the active filter query into the task link', () => {
    render(
      <MemoryRouter initialEntries={['/projects/project-1?tags=bug']}>
        <TaskCard projectId="project-1" task={task} showPriority={false} />
      </MemoryRouter>,
    );

    expect(screen.getByRole('link', { name: 'Task title' }).getAttribute('href')).toBe(
      '/projects/project-1/tasks/task-1?tags=bug',
    );
  });

  it('renders a drag handle only when the sortable provides one', () => {
    const { rerender } = render(
      <MemoryRouter>
        <TaskCard projectId="project-1" task={task} showPriority={false} />
      </MemoryRouter>,
    );

    expect(screen.queryByRole('button', { name: 'Drag Task title' })).toBeNull();

    rerender(
      <MemoryRouter>
        <TaskCard
          projectId="project-1"
          task={task}
          showPriority={false}
          dragHandleRef={() => undefined}
        />
      </MemoryRouter>,
    );

    expect(screen.getByRole('button', { name: 'Drag Task title' })).toBeDefined();
  });
});
