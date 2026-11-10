// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import { ProjectCard } from './ProjectCard';

const CREATED_AT = '2026-01-01T00:00:00Z';

function projectSnapshot(overrides: Partial<ProjectSnapshot> = {}): ProjectSnapshot {
  return {
    id: 'project-1',
    name: 'Project',
    description: 'A project description.',
    task_sorting_mode: 'manual',
    show_priority: false,
    created_at: CREATED_AT,
    sections: [
      {
        id: 'section-1',
        project_id: 'project-1',
        name: 'Backlog',
        position: 'a0',
        color: null,
        created_at: CREATED_AT,
      },
    ],
    tasks: [
      {
        id: 'task-1',
        project_id: 'project-1',
        title: 'Task',
        description: null,
        priority: 'medium',
        section_id: 'section-1',
        position: 'a0',
        state: 'open',
        tags: [],
        created_at: CREATED_AT,
      },
    ],
    notes: [],
    automations: [],
    ...overrides,
  };
}

afterEach(cleanup);

describe('ProjectCard', () => {
  it('renders the project name, description, and counts', () => {
    render(<ProjectCard project={projectSnapshot()} onUse={() => undefined} />);

    expect(screen.getByRole('heading', { name: 'Project' })).toBeDefined();
    expect(screen.getByText('A project description.')).toBeDefined();
    expect(screen.getByText('Sections')).toBeDefined();
    expect(screen.getByText('Tasks')).toBeDefined();
    expect(screen.getByText('Created')).toBeDefined();
  });

  it('calls the use action', () => {
    const onUse = vi.fn();
    render(<ProjectCard project={projectSnapshot()} onUse={onUse} />);

    fireEvent.click(screen.getByRole('button', { name: 'Use' }));

    expect(onUse).toHaveBeenCalledTimes(1);
  });
});
