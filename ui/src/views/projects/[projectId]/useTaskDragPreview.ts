import type { DragMoveEvent, DragOverEvent, DragStartEvent } from '@dnd-kit/react';
import { isSortable } from '@dnd-kit/react/sortable';
import { useRef, useState } from 'react';
import type { BoardGroup, DragLayout, TaskSlot } from './board';
import {
  groupContaining,
  groupKey,
  isNoSectionGroupKey,
  itemsFromGroups,
  previewAtPointer,
} from './board';

export type TaskDragPreview = {
  dragItems: Record<string, string[]> | null;
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
    const slots: TaskSlot[] = group.tasks.flatMap((task, index) => {
      const child = container.children.item(index);
      if (!(child instanceof HTMLElement)) {
        return [];
      }
      const rect = child.getBoundingClientRect();
      const top = rect.top - containerRect.top + scrollTop;
      return [{ id: task.id, top, bottom: top + rect.height }];
    });
    const gap = slots.length > 1 ? slots[1].top - slots[0].bottom : 0;
    layout.groups[key] = { gap, slots };
  }
  return layout;
}

export function useTaskDragPreview(groups: BoardGroup[]): TaskDragPreview {
  const [dragItems, setDragItems] = useState<Record<string, string[]> | null>(null);
  const dragLayoutRef = useRef<DragLayout | null>(null);
  const dragAnchorOffsetRef = useRef(0);

  function handleDragStart(event: DragStartEvent) {
    if (event.operation.source?.type === 'column') {
      return;
    }
    const sourceElement = event.operation.source?.element;
    const sourceRect =
      sourceElement instanceof HTMLElement ? sourceElement.getBoundingClientRect() : null;
    dragAnchorOffsetRef.current =
      sourceRect === null
        ? 0
        : sourceRect.top + sourceRect.height / 2 - event.operation.position.current.y;
    dragLayoutRef.current = captureDragLayout(groups, sourceRect?.height ?? 0);
    setDragItems(itemsFromGroups(groups));
  }

  function updateDragPreview(
    sourceId: string,
    pointer: { x: number; y: number },
    target: DragMoveEvent['operation']['target'],
  ): void {
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

    // Order by the dragged card's midpoint, not the cursor: the offset is the
    // grab point relative to the card center, captured when the drag started.
    const anchorY = pointer.y + dragAnchorOffsetRef.current;
    const orderY = anchorY - container.getBoundingClientRect().top + container.scrollTop;

    setDragItems((current) => {
      if (current === null) {
        return current;
      }
      const origin = groupContaining(current, String(sourceId));
      if (origin !== destination && isNoSectionGroupKey(destination)) {
        return current;
      }
      return previewAtPointer(current, layout, destination, String(sourceId), orderY);
    });
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
    dragAnchorOffsetRef.current = 0;
    setDragItems(null);
  }

  return { dragItems, handleDragStart, handleDragMove, handleDragOver, resetDragPreview };
}
