import { CollisionPriority } from '@dnd-kit/abstract';
import { useDroppable } from '@dnd-kit/react';
import { Badge, Card } from 'flowbite-react';
import type { CardTheme } from 'flowbite-react';
import type { DeepPartial } from 'flowbite-react/types';
import { AnimatePresence, motion } from 'framer-motion';
import { useState } from 'react';
import { HiChevronDown, HiChevronUp, HiLockClosed, HiTrash } from 'react-icons/hi2';
import { RenamableLabel } from '@/components/RenamableLabel';
import type { Color } from '@/generated/Color';
import type { ExternalIdentity } from '@/generated/ExternalIdentity';
import type { Task } from '@/generated/Task';
import { BoardColumnBody } from './BoardColumnBody';
import { DeleteSectionModal } from './DeleteSectionModal';
import { SectionColorPicker } from './SectionColorPicker';

export type BoardColumnProps = {
  projectId: string;
  group: string;
  droppable?: boolean;
  title: string;
  color: Color | null;
  tasks: Task[];
  showPriority: boolean;
  canReorder: boolean;
  blockingAutomations?: readonly string[];
  onRename?: (name: string) => void;
  onDelete?: () => void;
  onCreate?: (title: string, externalIdentity: ExternalIdentity | null) => Promise<void>;
  showCreate?: boolean;
  onColorChange?: (color: Color | null) => void;
  nodeRef?: (element: HTMLDivElement | null) => void;
  isDragging?: boolean;
  draggable?: boolean;
  collapsed?: boolean;
  onToggleCollapsed?: () => void;
};

const columnTheme: DeepPartial<CardTheme> = {
  root: {
    children: 'flex h-full min-h-0 flex-col justify-start gap-2 p-2',
  },
};

export function BoardColumn(props: BoardColumnProps) {
  const {
    projectId,
    group,
    droppable,
    title,
    color,
    tasks,
    showPriority,
    canReorder,
    blockingAutomations = [],
    onRename,
    onDelete,
    onCreate,
    showCreate = true,
    onColorChange,
    nodeRef,
    isDragging,
    draggable = false,
    collapsed = false,
    onToggleCollapsed,
  } = props;

  const [confirmingDelete, setConfirmingDelete] = useState(false);
  const bodyId = `${group}-tasks`;
  const collapsible = onToggleCollapsed !== undefined;

  const { ref, isDropTarget } = useDroppable({
    id: group,
    type: 'column',
    accept: 'item',
    collisionPriority: CollisionPriority.Low,
    disabled: !(droppable ?? true) || collapsed,
  });

  function stopRowToggle(event: React.SyntheticEvent) {
    event.stopPropagation();
  }

  function handleToggle() {
    onToggleCollapsed?.();
  }

  return (
    <>
      <Card
        ref={nodeRef}
        theme={columnTheme}
        className={`flex min-h-0 w-full min-w-0 shrink-0 flex-col md:h-full md:w-96 ${
          draggable ? 'cursor-grab active:cursor-grabbing' : ''
        } ${isDragging ? 'opacity-80' : ''}`}
      >
        <div
          role={collapsible ? 'button' : undefined}
          tabIndex={collapsible ? 0 : undefined}
          aria-label={
            collapsible ? (collapsed ? `Expand ${title}` : `Collapse ${title}`) : undefined
          }
          aria-expanded={collapsible ? !collapsed : undefined}
          onClick={collapsible ? handleToggle : undefined}
          onKeyDown={(event) => {
            if (!collapsible) {
              return;
            }
            if (event.key === 'Enter' || event.key === ' ') {
              event.preventDefault();
              handleToggle();
            }
          }}
          className={`flex shrink-0 items-center justify-between gap-2 ${
            collapsible
              ? 'focus-visible:outline-line-hover cursor-pointer select-none focus:outline-none focus-visible:outline-2 focus-visible:outline-offset-2'
              : ''
          }`}
        >
          <div className="flex min-w-0 items-center gap-2">
            {onColorChange === undefined ? null : (
              <span onClick={stopRowToggle} onKeyDown={stopRowToggle}>
                <SectionColorPicker color={color} onChange={onColorChange} />
              </span>
            )}
            <RenamableLabel
              label={title}
              onRename={onRename}
              labelAs="h2"
              inputAriaLabel="Section name"
            />
          </div>
          <div className="flex shrink-0 items-center gap-1">
            <Badge color="gray">{tasks.length}</Badge>
            {blockingAutomations.length === 0 ? null : (
              <HiLockClosed
                aria-label="Used by an automation"
                title="This section is used by an automation."
                className="text-muted h-3.5 w-3.5"
              />
            )}
            {onDelete === undefined ? null : (
              <button
                type="button"
                aria-label={`Delete ${title}`}
                title={`Delete ${title}`}
                onClick={(event) => {
                  stopRowToggle(event);
                  setConfirmingDelete(true);
                }}
                onKeyDown={stopRowToggle}
                className="text-muted inline-flex h-6 w-6 shrink-0 cursor-pointer items-center justify-center border-0 bg-transparent p-0 transition-colors hover:text-red-400 focus:outline-none focus-visible:outline-none"
              >
                <HiTrash aria-hidden="true" className="h-3.5 w-3.5" />
              </button>
            )}
            {collapsible ? (
              collapsed ? (
                <HiChevronDown aria-hidden="true" className="text-muted h-4 w-4 shrink-0" />
              ) : (
                <HiChevronUp aria-hidden="true" className="text-muted h-4 w-4 shrink-0" />
              )
            ) : null}
          </div>
        </div>
        {collapsible ? (
          <AnimatePresence initial={false}>
            {collapsed ? null : (
              <motion.div
                id={bodyId}
                initial={{ height: 0, opacity: 0 }}
                animate={{ height: 'auto', opacity: 1 }}
                exit={{ height: 0, opacity: 0 }}
                transition={{ duration: 0.25, ease: 'easeInOut' }}
                className="overflow-hidden"
              >
                <div className="flex min-h-0 flex-col gap-2">
                  <BoardColumnBody
                    projectId={projectId}
                    group={group}
                    tasks={tasks}
                    showPriority={showPriority}
                    canReorder={canReorder}
                    showCreate={showCreate}
                    onCreate={onCreate}
                    dropRef={ref}
                    isDropTarget={isDropTarget}
                  />
                </div>
              </motion.div>
            )}
          </AnimatePresence>
        ) : (
          <div id={bodyId} className="flex min-h-0 flex-1 flex-col gap-2">
            <BoardColumnBody
              projectId={projectId}
              group={group}
              tasks={tasks}
              showPriority={showPriority}
              canReorder={canReorder}
              showCreate={showCreate}
              onCreate={onCreate}
              dropRef={ref}
              isDropTarget={isDropTarget}
            />
          </div>
        )}
      </Card>
      {onDelete === undefined ? null : (
        <DeleteSectionModal
          sectionName={title}
          blockingAutomations={blockingAutomations}
          show={confirmingDelete}
          onClose={() => setConfirmingDelete(false)}
          onDelete={() => {
            setConfirmingDelete(false);
            onDelete();
          }}
        />
      )}
    </>
  );
}
