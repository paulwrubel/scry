import { describe, expect, it } from 'vitest';
import type { Task } from '../generated/Task';
import type { TaskSortingMode } from '../generated/TaskSortingMode';
import type { FixtureCase } from '../fixtures/load';
import { loadArea } from '../fixtures/load';
import { compareTasks } from './task-order';

const taskOrder = loadArea('task-order');

describe('task order', () => {
  for (const testCase of taskOrder.cases) {
    it(testCase.name, () => {
      runCase(testCase);
    });
  }
});

function runCase(testCase: FixtureCase): void {
  switch (testCase.kind) {
    case 'sort':
      runSort(testCase);
      return;
    default:
      throw new Error(`case "${testCase.name}" uses unknown fixture kind "${testCase.kind}"`);
  }
}

function runSort(testCase: FixtureCase): void {
  const mode = testCase.input.mode as TaskSortingMode;
  const tasks = testCase.input.tasks as unknown as Task[];
  const sorted = [...tasks].sort((left, right) => compareTasks(mode, left, right));

  expect(sorted.map((task) => task.id)).toEqual(testCase.expected.ids);
}
