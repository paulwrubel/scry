import type { DragMoveEvent, DragOverEvent, DragStartEvent } from '@dnd-kit/react';
import { isSortable } from '@dnd-kit/react/sortable';
import { useRef, useState } from 'react';
import type { Priority } from '@/generated/Priority';
import type { BoardGroup, BoundarySide, DragLayout, SoloHome, TaskSlot } from './board';
import {
  boundarySideForSlot,
  groupContaining,
  groupKey,
  isNoSectionGroupKey,
  itemsFromGroups,
  previewAtPointerDetailed,
  soloPriorityHome,
} from './board';

export type TaskDragPreview = {
  dragItems: Record<string, string[]> | null;
  draggedTaskId: string | null;
  boundarySide: BoundarySide | null;
  handleDragStart: (event: DragStartEvent) => void;
  handleDragMove: (event: DragMoveEvent) => void;
  handleDragOver: (event: DragOverEvent) => void;
  resetDragPreview: () => void;
};

function dragPointer(event: DragMoveEvent): { x: number; y: number } {
  if (event.to !== undefined) {
    return event.to;
  }
  const position = event.operation.position.current;
  return event.by === undefined
    ? position
    : { x: position.x + event.by.x, y: position.y + event.by.y };
}

function captureDragLayout(groups: BoardGroup[], draggedHeight: number): DragLayout {
  const layout: DragLayout = { groups: {}, draggedHeight };
  for (const group of groups) {
    const key = groupKey(group);
    const container = document.querySelector<HTMLElement>(
      `[data-board-group="${CSS.escape(key)}"]`,
    );
    if (container === null) {
      continue;
    }
    const containerRect = container.getBoundingClientRect();
    const scrollTop = container.scrollTop;
    const elements = new Map<string, HTMLElement>();
    for (const element of container.querySelectorAll<HTMLElement>('[data-task-id]')) {
      const taskId = element.dataset.taskId;
      if (taskId !== undefined) {
        elements.set(taskId, element);
      }
    }
    const slots: TaskSlot[] = group.tasks.flatMap((task) => {
      const element = elements.get(task.id);
      if (element === undefined) {
        return [];
      }
      const rect = element.getBoundingClientRect();
      const top = rect.top - containerRect.top + scrollTop;
      return [{ id: task.id, top, bottom: top + rect.height }];
    });
    const gap = slots.length > 1 ? slots[1].top - slots[0].bottom : 0;
    layout.groups[key] = { gap, slots };
  }
  return layout;
}

export function useTaskDragPreview(groups: BoardGroup[], priorityView: boolean): TaskDragPreview {
  const [dragItems, setDragItems] = useState<Record<string, string[]> | null>(null);
  const [draggedTaskId, setDraggedTaskId] = useState<string | null>(null);
  const [boundarySide, setBoundarySide] = useState<BoundarySide | null>(null);
  const dragLayoutRef = useRef<DragLayout | null>(null);
  const dragAnchorOffsetRef = useRef(0);
  const dragAnchorOffsetXRef = useRef(0);
  const dragItemsRef = useRef<Record<string, string[]> | null>(null);
  const baseItemsRef = useRef<Record<string, string[]> | null>(null);
  const soloHomeRef = useRef<{
    home: SoloHome;
    container: HTMLElement;
    priority: Priority;
  } | null>(null);
  const homeLatchRef = useRef<{
    top: number;
    bottom: number;
    left: number;
    right: number;
  } | null>(null);

  function handleDragStart(event: DragStartEvent) {
    if (event.operation.source?.type === 'column') {
      return;
    }
    const sourceId = event.operation.source?.id;
    setDraggedTaskId(sourceId === undefined ? null : String(sourceId));
    const sourceElement = event.operation.source?.element;
    const sourceRect =
      sourceElement instanceof HTMLElement ? sourceElement.getBoundingClientRect() : null;
    dragAnchorOffsetRef.current =
      sourceRect === null
        ? 0
        : sourceRect.top + sourceRect.height / 2 - event.operation.position.current.y;
    dragAnchorOffsetXRef.current =
      sourceRect === null
        ? 0
        : sourceRect.left + sourceRect.width / 2 - event.operation.position.current.x;
    dragLayoutRef.current = captureDragLayout(groups, sourceRect?.height ?? 0);
    setBoundarySide(null);
    const items = itemsFromGroups(groups);
    dragItemsRef.current = items;
    baseItemsRef.current = items;
    setDragItems(items);
    soloHomeRef.current = null;
    homeLatchRef.current = null;
    if (priorityView && sourceId !== undefined) {
      const group = groups.find((candidate) =>
        candidate.tasks.some((task) => task.id === String(sourceId)),
      );
      if (group !== undefined) {
        const task = group.tasks.find((candidate) => candidate.id === String(sourceId));
        const home = soloPriorityHome(group, String(sourceId), dragLayoutRef.current);
        const container = document.querySelector<HTMLElement>(
          `[data-board-group="${CSS.escape(groupKey(group))}"]`,
        );
        soloHomeRef.current =
          home === null || container === null || task === undefined
            ? null
            : { home, container, priority: task.priority };
      }
    }
  }

  function updateDragPreview(
    sourceId: string,
    pointer: { x: number; y: number },
    target: DragMoveEvent['operation']['target'],
  ): void {
    // Order by the dragged card's midpoint, not the cursor: the offset is the
    // grab point relative to the card center, captured when the drag started.
    const anchorY = pointer.y + dragAnchorOffsetRef.current;
    const anchorX = pointer.x + dragAnchorOffsetXRef.current;

    // a solo priority band disappears once its only card leaves, so its
    // rendered ghost band stays a live target for dropping the card back
    const home = soloHomeRef.current;
    if (home !== null) {
      const containerRect = home.container.getBoundingClientRect();
      const originalTop = containerRect.top - home.container.scrollTop + home.home.top;
      const originalBottom = originalTop + home.home.height;
      const ghost = home.container.querySelector<HTMLElement>(
        `[data-ghost-band="${home.priority}"]`,
      );
      const ghostRect = ghost?.getBoundingClientRect();
      // the preview reflow moves the ghost, so latch the union of the original
      // slot and the rendered ghost until the pointer leaves it; otherwise the
      // source band would reappear and disappear under the same pointer
      const bounds = {
        top: ghostRect === undefined ? originalTop : Math.min(originalTop, ghostRect.top - 8),
        bottom:
          ghostRect === undefined ? originalBottom : Math.max(originalBottom, ghostRect.bottom),
        left: containerRect.left,
        right: containerRect.right,
      };
      const inside = (area: typeof bounds) =>
        anchorX >= area.left &&
        anchorX <= area.right &&
        anchorY >= area.top &&
        anchorY <= area.bottom;
      const latch = homeLatchRef.current;
      if (latch !== null && !inside(latch)) {
        homeLatchRef.current = null;
      } else {
        const homeArea = latch ?? bounds;
        if (inside(homeArea)) {
          homeLatchRef.current = homeArea;
          if (baseItemsRef.current !== null) {
            dragItemsRef.current = baseItemsRef.current;
            setDragItems(baseItemsRef.current);
          }
          setBoundarySide(null);
          return;
        }
      }
    }

    const layout = dragLayoutRef.current;
    if (layout === null || target === null) {
      return;
    }

    let destination: string | null = null;
    if (isSortable(target)) {
      destination = target.group === undefined ? null : String(target.group);
    } else if (target.type === 'column') {
      destination = String(target.id);
    }
    if (destination === null) {
      return;
    }

    const container = target.element?.closest('[data-board-group]');
    if (!(container instanceof HTMLElement)) {
      return;
    }

    const orderY = anchorY - container.getBoundingClientRect().top + container.scrollTop;

    const current = dragItemsRef.current;
    if (current === null) {
      return;
    }
    const origin = groupContaining(current, String(sourceId));
    if (origin !== destination && isNoSectionGroupKey(destination)) {
      return;
    }
    const result = previewAtPointerDetailed(current, layout, destination, String(sourceId), orderY);
    dragItemsRef.current = result.items;
    setDragItems(result.items);
    setBoundarySide(boundarySideForSlot(result.slotCenter, orderY));
  }

  function handleDragMove(event: DragMoveEvent) {
    if (event.operation.source?.type === 'column') {
      return;
    }

    const sourceId = event.operation.source?.id;
    if (sourceId === undefined) {
      return;
    }

    updateDragPreview(String(sourceId), dragPointer(event), event.operation.target);
  }

  function handleDragOver(event: DragOverEvent) {
    if (event.operation.source?.type === 'column') {
      return;
    }
    event.preventDefault();

    const sourceId = event.operation.source?.id;
    if (sourceId === undefined) {
      return;
    }

    updateDragPreview(String(sourceId), event.operation.position.current, event.operation.target);
  }

  function resetDragPreview() {
    dragLayoutRef.current = null;
    dragItemsRef.current = null;
    baseItemsRef.current = null;
    soloHomeRef.current = null;
    homeLatchRef.current = null;
    dragAnchorOffsetRef.current = 0;
    dragAnchorOffsetXRef.current = 0;
    setDragItems(null);
    setDraggedTaskId(null);
    setBoundarySide(null);
  }

  return {
    dragItems,
    draggedTaskId,
    boundarySide,
    handleDragStart,
    handleDragMove,
    handleDragOver,
    resetDragPreview,
  };
}
