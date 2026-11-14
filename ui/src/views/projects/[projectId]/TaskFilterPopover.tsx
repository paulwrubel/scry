import {
  Badge,
  Button,
  Checkbox,
  Label,
  ListGroup,
  Popover,
  Radio,
  TextInput,
} from 'flowbite-react';
import type { PopoverTheme } from 'flowbite-react';
import type { DeepPartial } from 'flowbite-react/types';
import { useMemo } from 'react';
import { HiFunnel } from 'react-icons/hi2';
import type { Task } from '@/generated/Task';
import { tagBadgeStyle } from '@/utils/tasks/tagColor';
import {
  TASK_FILTER_STATUS_OPTIONS,
  activeFilterCount,
  collectProjectTags,
  countTasksByTag,
  setTaskFilterSearch,
  setTaskFilterStatus,
  taskFilterStatus,
  toggleTaskFilterTag,
} from '@/utils/tasks/filter';
import type { TaskFilter } from '@/utils/tasks/filter';

const popoverTheme: DeepPartial<PopoverTheme> = {
  base: 'absolute z-30 w-80 rounded-lg border border-line bg-panel shadow-lg outline-none',
  content: 'overflow-hidden rounded-lg',
};

export type TaskFilterPopoverProps = {
  filter: TaskFilter;
  tasks: readonly Task[];
  onChange: (filter: TaskFilter) => void;
  onClear: () => void;
};

export function TaskFilterPopover(props: TaskFilterPopoverProps) {
  const { filter, tasks, onChange, onClear } = props;

  const tags = useMemo(() => collectProjectTags(tasks, filter.tags), [tasks, filter.tags]);
  const counts = useMemo(() => countTasksByTag(tasks), [tasks]);
  const activeCount = activeFilterCount(filter);

  return (
    <Popover
      arrow={false}
      placement="bottom-end"
      trigger="click"
      theme={popoverTheme}
      content={
        <div className="flex flex-col gap-4 p-4">
          <div className="flex flex-col gap-2">
            <Label htmlFor="task-filter-search">Search</Label>
            <TextInput
              id="task-filter-search"
              sizing="sm"
              aria-label="Filter by text"
              placeholder="Title or description"
              value={filter.search}
              onChange={(event) => onChange(setTaskFilterSearch(filter, event.target.value))}
            />
          </div>
          <fieldset className="flex flex-col gap-2">
            <legend className="text-muted text-xs font-semibold tracking-wide uppercase">
              Status
            </legend>
            <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
              {TASK_FILTER_STATUS_OPTIONS.map((option) => (
                <Label
                  key={option.value}
                  htmlFor={`task-filter-status-${option.value}`}
                  className="flex cursor-pointer items-center gap-2 text-sm font-normal"
                >
                  <Radio
                    id={`task-filter-status-${option.value}`}
                    name="task-filter-status"
                    value={option.value}
                    checked={taskFilterStatus(filter) === option.value}
                    aria-label={option.label}
                    onChange={() => onChange(setTaskFilterStatus(filter, option.value))}
                  />
                  <span>{option.label}</span>
                </Label>
              ))}
            </div>
          </fieldset>
          <fieldset className="flex flex-col gap-2">
            <legend className="text-muted text-xs font-semibold tracking-wide uppercase">
              Tags
            </legend>
            {tags.length === 0 ? (
              <p className="text-faint text-sm">No tags on this project.</p>
            ) : (
              <ListGroup className="border-line bg-surface max-h-48 overflow-y-auto text-sm font-medium text-zinc-200 select-none">
                {tags.map((tag, index) => (
                  <li key={tag} className="border-line w-full border-b last:border-b-0">
                    <div className="flex items-center ps-3">
                      <Checkbox
                        id={`task-filter-tag-${index}`}
                        aria-label={tag}
                        checked={filter.tags.includes(tag)}
                        onChange={() => onChange(toggleTaskFilterTag(filter, tag))}
                      />
                      <Label
                        htmlFor={`task-filter-tag-${index}`}
                        className="flex w-full cursor-pointer items-center gap-2 py-3 ps-2 pe-3 text-sm font-medium"
                      >
                        <Badge color="gray" style={tagBadgeStyle(tag)}>
                          {tag}
                        </Badge>
                        <span className="text-faint ms-auto text-xs">{counts.get(tag) ?? 0}</span>
                      </Label>
                    </div>
                  </li>
                ))}
              </ListGroup>
            )}
          </fieldset>
          <div className="border-line flex items-center justify-between gap-2 border-t pt-3">
            <span className="text-faint text-xs">
              {activeCount === 0 ? 'No filters' : `${activeCount} active`}
            </span>
            <Button
              type="button"
              color="light"
              size="xs"
              onClick={onClear}
              disabled={activeCount === 0}
            >
              Clear all
            </Button>
          </div>
        </div>
      }
    >
      <Button
        type="button"
        aria-label="Filter tasks"
        title="Filter tasks"
        color="alternative"
        size="xs"
        className="min-h-11 min-w-11 justify-center gap-1.5 px-0 md:min-h-0 md:min-w-0 md:px-3"
      >
        <HiFunnel aria-hidden="true" className="h-4 w-4" />
        <span className="hidden md:inline">Filter</span>
        {activeCount > 0 ? <Badge color="gray">{activeCount}</Badge> : null}
      </Button>
    </Popover>
  );
}
