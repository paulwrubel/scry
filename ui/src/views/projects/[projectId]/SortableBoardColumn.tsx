import { useSortable } from '@dnd-kit/react/sortable';
import type { Color } from '@/generated/Color';
import type { ExternalIdentity } from '@/generated/ExternalIdentity';
import type { Priority } from '@/generated/Priority';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Section } from '@/generated/Section';
import { automationReferencesSection, describeAutomation } from '@/utils/automationOperations';
import { BoardColumn } from './BoardColumn';
import { columnDragId, groupKey, type BoardGroup } from './board';

export type SectionBoardGroup = BoardGroup & { section: Section };

export type SortableBoardColumnProps = {
  group: SectionBoardGroup;
  index: number;
  project: ProjectSnapshot;
  sectionsSortable: boolean;
  showCreate: boolean;
  priorityView: boolean;
  ghostPriority: Priority | null;
  dragTarget: { taskId: string; priority: Priority } | null;
  collapsed: boolean;
  onToggleCollapsed?: () => void;
  onCreate: (
    sectionId: string,
    title: string,
    externalIdentity: ExternalIdentity | null,
  ) => Promise<void>;
  onRename: (sectionId: string, name: string) => void;
  onColorChange: (sectionId: string, color: Color | null) => void;
  onDelete: (sectionId: string) => void;
};

export function SortableBoardColumn(props: SortableBoardColumnProps) {
  const {
    group,
    index,
    project,
    sectionsSortable,
    showCreate,
    priorityView,
    ghostPriority,
    dragTarget,
    collapsed,
    onToggleCollapsed,
    onCreate,
    onRename,
    onColorChange,
    onDelete,
  } = props;

  const section = group.section;
  const blockingAutomations = project.automations
    .filter((automation) => automationReferencesSection(automation, section.id))
    .map((automation) => describeAutomation(automation, project.sections));
  const { ref, isDragging } = useSortable({
    id: columnDragId(section.id),
    index,
    group: 'columns',
    type: 'column',
    accept: 'column',
    disabled: !sectionsSortable,
  });

  return (
    <BoardColumn
      nodeRef={(element) => ref(element)}
      isDragging={isDragging}
      draggable={sectionsSortable}
      projectId={project.id}
      group={groupKey(group)}
      title={section.name}
      color={section.color}
      tasks={group.tasks}
      showPriority={project.show_priority}
      priorityView={priorityView}
      ghostPriority={ghostPriority}
      dragTarget={dragTarget}
      blockingAutomations={blockingAutomations}
      showCreate={showCreate}
      collapsed={collapsed}
      onToggleCollapsed={onToggleCollapsed}
      onRename={(name) => onRename(section.id, name)}
      onColorChange={(color) => onColorChange(section.id, color)}
      onDelete={() => onDelete(section.id)}
      onCreate={(title, identity) => onCreate(section.id, title, identity)}
    />
  );
}
