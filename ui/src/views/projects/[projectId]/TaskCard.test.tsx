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

  it('renders a plain description preview and preserves newlines', () => {
    render(
      <MemoryRouter initialEntries={['/projects/project-1?tags=bug']}>
        <TaskCard
          projectId="project-1"
          task={{ ...task, description: 'First line\nSecond line' }}
          showPriority={false}
        />
      </MemoryRouter>,
    );

    const preview = screen.getByText('First line Second line');
    expect(preview.textContent).toBe('First line\nSecond line');
    expect(preview.className).toContain('whitespace-pre-wrap');
    expect(screen.getAllByRole('link')).toHaveLength(1);
  });

  it('renders the priority dot at the top-right of the title row', () => {
    render(
      <MemoryRouter initialEntries={['/projects/project-1?tags=bug']}>
        <TaskCard projectId="project-1" task={task} showPriority={true} />
      </MemoryRouter>,
    );

    const link = screen.getByRole('link', { name: 'Task title' });
    expect(link.nextElementSibling).toBe(screen.getByRole('img', { name: 'medium priority' }));
  });

  it('renders no extra description text for a null description', () => {
    render(
      <MemoryRouter initialEntries={['/projects/project-1?tags=bug']}>
        <TaskCard projectId="project-1" task={task} showPriority={false} />
      </MemoryRouter>,
    );

    expect(screen.getAllByRole('link')).toHaveLength(1);
  });
});
