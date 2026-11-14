// @vitest-environment jsdom
import { cleanup, render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, describe, expect, it } from 'vitest';
import type { ExternalIdentity } from '@/generated/ExternalIdentity';
import type { ExternalMetadata } from '@/generated/ExternalMetadata';
import type { Task } from '@/generated/Task';
import { TaskCard } from './TaskCard';

const CREATED_AT = '2026-01-01T00:00:00Z';

const GITHUB_IDENTITY: ExternalIdentity = {
  provider: 'github',
  owner: 'acme',
  repo: 'widgets',
  number: 123,
};

function metadata(overrides: Partial<ExternalMetadata> = {}): ExternalMetadata {
  return {
    provider: 'github',
    title: 'Issue title',
    description: 'Issue body',
    kind: 'issue',
    state: 'open',
    reason: null,
    draft: false,
    updated_at: '2026-02-01T12:00:00Z',
    error: null,
    ...overrides,
  };
}

const task: Task = {
  id: 'task-1',
  project_id: 'project-1',
  title: 'Task title',
  description: null,
  priority: 'normal',
  section_id: null,
  position: 'a0',
  state: 'open',
  tags: [],
  created_at: CREATED_AT,
  updated_at: CREATED_AT,
  external_identity: null,
  external_metadata: null,
};

afterEach(cleanup);

describe('TaskCard', () => {
  it('shows a grab cursor when draggable', () => {
    const { container } = render(
      <MemoryRouter>
        <TaskCard projectId="project-1" task={task} showPriority={false} draggable />
      </MemoryRouter>,
    );

    expect(container.firstElementChild?.className).toContain('cursor-grab');
  });

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
        <TaskCard
          projectId="project-1"
          task={{ ...task, priority: 'elevated' }}
          showPriority={true}
        />
      </MemoryRouter>,
    );

    const link = screen.getByRole('link', { name: 'Task title' });
    const controls = link.nextElementSibling;
    expect(controls).not.toBeNull();
    expect(controls?.contains(screen.getByRole('img', { name: 'elevated priority' }))).toBe(true);
  });

  it('renders no priority dot for a normal task', () => {
    render(
      <MemoryRouter initialEntries={['/projects/project-1?tags=bug']}>
        <TaskCard projectId="project-1" task={task} showPriority={true} />
      </MemoryRouter>,
    );

    expect(screen.queryByRole('img', { name: 'normal priority' })).toBeNull();
    expect(screen.queryAllByRole('img')).toHaveLength(0);
  });

  it('renders no extra description text for a null description', () => {
    render(
      <MemoryRouter initialEntries={['/projects/project-1?tags=bug']}>
        <TaskCard projectId="project-1" task={task} showPriority={false} />
      </MemoryRouter>,
    );

    expect(screen.getAllByRole('link')).toHaveLength(1);
  });

  it('renders a GitHub issue link for a linked task', () => {
    render(
      <MemoryRouter initialEntries={['/projects/project-1']}>
        <TaskCard
          projectId="project-1"
          task={{ ...task, external_identity: GITHUB_IDENTITY, external_metadata: metadata() }}
          showPriority={false}
        />
      </MemoryRouter>,
    );

    expect(screen.getByRole('link', { name: 'Open GitHub issue' }).getAttribute('href')).toBe(
      'https://github.com/acme/widgets/issues/123',
    );
  });

  it('renders a GitHub pull request link for a linked pull request', () => {
    render(
      <MemoryRouter initialEntries={['/projects/project-1']}>
        <TaskCard
          projectId="project-1"
          task={{
            ...task,
            external_identity: GITHUB_IDENTITY,
            external_metadata: metadata({ kind: 'pull' }),
          }}
          showPriority={false}
        />
      </MemoryRouter>,
    );

    expect(
      screen.getByRole('link', { name: 'Open GitHub pull request' }).getAttribute('href'),
    ).toBe('https://github.com/acme/widgets/pull/123');
  });

  it('colors the GitHub icon by external state', () => {
    const { rerender } = render(
      <MemoryRouter>
        <TaskCard
          projectId="project-1"
          task={{
            ...task,
            external_identity: GITHUB_IDENTITY,
            external_metadata: metadata({ state: 'open' }),
          }}
          showPriority={false}
        />
      </MemoryRouter>,
    );

    expect(screen.getByRole('link', { name: 'Open GitHub issue' }).className).toContain(
      'text-green-400',
    );

    rerender(
      <MemoryRouter>
        <TaskCard
          projectId="project-1"
          task={{
            ...task,
            external_identity: GITHUB_IDENTITY,
            external_metadata: metadata({ state: 'closed' }),
          }}
          showPriority={false}
        />
      </MemoryRouter>,
    );

    expect(screen.getByRole('link', { name: 'Open GitHub issue' }).className).toContain(
      'text-purple-400',
    );

    rerender(
      <MemoryRouter>
        <TaskCard
          projectId="project-1"
          task={{
            ...task,
            external_identity: GITHUB_IDENTITY,
            external_metadata: metadata({ kind: 'pull', state: 'closed' }),
          }}
          showPriority={false}
        />
      </MemoryRouter>,
    );

    expect(screen.getByRole('link', { name: 'Open GitHub pull request' }).className).toContain(
      'text-red-400',
    );
  });

  it('renders the GitHub error badge', () => {
    render(
      <MemoryRouter initialEntries={['/projects/project-1']}>
        <TaskCard
          projectId="project-1"
          task={{
            ...task,
            external_identity: GITHUB_IDENTITY,
            external_metadata: metadata({ error: 'not_found' }),
          }}
          showPriority={false}
        />
      </MemoryRouter>,
    );

    expect(screen.getByText('GitHub: not_found')).toBeDefined();
  });
});
