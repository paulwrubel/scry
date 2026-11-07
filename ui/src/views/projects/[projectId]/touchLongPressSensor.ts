import { Sensor } from '@dnd-kit/abstract';
import type { DragDropManager, Draggable } from '@dnd-kit/dom';

const LONG_PRESS_DELAY = 300;
const MOVE_TOLERANCE = 8;

type Point = { x: number; y: number };

function pointOf(touch: Touch): Point {
  return { x: touch.clientX, y: touch.clientY };
}

function findTouch(touches: TouchList, identifier: number): Touch | undefined {
  for (let index = 0; index < touches.length; index += 1) {
    if (touches[index]?.identifier === identifier) {
      return touches[index];
    }
  }

  return undefined;
}

function movedBeyond(start: Point, next: Point): boolean {
  return Math.hypot(next.x - start.x, next.y - start.y) > MOVE_TOLERANCE;
}

/**
 * Touch Events can prevent scrolling in touchmove; Pointer Events cannot.
 * A short hold arms the drag, and movement before that is allowed to scroll.
 */
export class TouchLongPressSensor extends Sensor<DragDropManager> {
  bind(source: Draggable): () => void {
    const element = source.element;
    if (!element || source.type !== 'item') {
      return () => undefined;
    }

    let timer: ReturnType<typeof setTimeout> | undefined;
    let clickGuardTimer: ReturnType<typeof setTimeout> | undefined;
    let start: Point | undefined;
    let startTouchId: number | undefined;
    let latestEvent: TouchEvent | undefined;
    let active = false;
    let document: Document | undefined;

    const suppressClick = (event: Event) => {
      event.preventDefault();
      event.stopImmediatePropagation();
    };

    const preventContextMenu = (event: Event) => {
      event.preventDefault();
    };

    const removeDocumentListeners = () => {
      if (!document) {
        return;
      }

      document.removeEventListener('touchmove', handleMove, true);
      document.removeEventListener('touchend', handleEnd, true);
      document.removeEventListener('touchcancel', handleCancel, true);
      document.removeEventListener('pointerup', handlePointerEnd, true);
      document.removeEventListener('pointercancel', handlePointerCancel, true);
      document.removeEventListener('contextmenu', preventContextMenu, true);
    };

    const clearTimer = () => {
      if (timer !== undefined) {
        clearTimeout(timer);
        timer = undefined;
      }
    };

    const armClickGuard = () => {
      if (!document) {
        return;
      }

      document.addEventListener('click', suppressClick, true);
      if (clickGuardTimer !== undefined) {
        clearTimeout(clickGuardTimer);
      }
      clickGuardTimer = setTimeout(() => {
        document?.removeEventListener('click', suppressClick, true);
        clickGuardTimer = undefined;
      }, 400);
    };

    const cleanup = (removeClickGuard = false) => {
      clearTimer();
      removeDocumentListeners();

      if (removeClickGuard && document) {
        document.removeEventListener('click', suppressClick, true);
        if (clickGuardTimer !== undefined) {
          clearTimeout(clickGuardTimer);
          clickGuardTimer = undefined;
        }
      }

      start = undefined;
      startTouchId = undefined;
      latestEvent = undefined;
      active = false;
    };

    const handleMove = (event: TouchEvent) => {
      if (start === undefined || startTouchId === undefined) {
        return;
      }

      const touch = findTouch(event.touches, startTouchId);
      if (touch === undefined) {
        return;
      }

      const next = pointOf(touch);
      if (!active) {
        if (movedBeyond(start, next)) {
          cleanup();
        }
        return;
      }

      event.preventDefault();

      const hit = element.ownerDocument.elementFromPoint(next.x, next.y);
      if (hit !== null && hit.closest('nav[aria-label="Primary"]') !== null) {
        try {
          this.manager.actions.stop({ event, canceled: true });
        } finally {
          cleanup();
        }
        return;
      }

      latestEvent = event;
      this.manager.actions.move({ event, to: next });
    };

    const handleEnd = (event: TouchEvent) => {
      if (active) {
        event.preventDefault();
        try {
          this.manager.actions.stop({ event });
        } finally {
          armClickGuard();
        }
      }

      cleanup();
    };

    const handlePointerEnd = (event: PointerEvent) => {
      if (!active) {
        return;
      }

      try {
        this.manager.actions.stop({ event });
      } finally {
        armClickGuard();
        cleanup();
      }
    };

    const handlePointerCancel = (event: PointerEvent) => {
      if (!active) {
        return;
      }

      try {
        this.manager.actions.stop({ event, canceled: true });
      } finally {
        cleanup();
      }
    };

    const handleCancel = (event: TouchEvent) => {
      if (active) {
        try {
          this.manager.actions.stop({ event, canceled: true });
        } finally {
          cleanup();
        }
        return;
      }

      cleanup();
    };

    const handleStart = (event: TouchEvent) => {
      if (this.disabled || source.disabled || !this.manager.dragOperation.status.idle) {
        return;
      }

      if (event.touches.length !== 1) {
        return;
      }

      const touch = event.touches[0];
      if (touch === undefined) {
        return;
      }

      start = pointOf(touch);
      startTouchId = touch.identifier;
      latestEvent = event;
      const ownerDocument = element.ownerDocument;
      document = ownerDocument;

      ownerDocument.addEventListener('touchmove', handleMove, { passive: false, capture: true });
      ownerDocument.addEventListener('touchend', handleEnd, { passive: false, capture: true });
      ownerDocument.addEventListener('touchcancel', handleCancel, {
        passive: false,
        capture: true,
      });
      ownerDocument.addEventListener('pointerup', handlePointerEnd, { capture: true });
      ownerDocument.addEventListener('pointercancel', handlePointerCancel, { capture: true });
      ownerDocument.addEventListener('contextmenu', preventContextMenu, true);

      clearTimer();
      timer = setTimeout(() => {
        timer = undefined;
        if (
          this.disabled ||
          source.disabled ||
          !this.manager.dragOperation.status.idle ||
          start === undefined
        ) {
          cleanup();
          return;
        }

        const controller = this.manager.actions.start({
          event: latestEvent ?? event,
          source,
          coordinates: start,
        });

        if (controller.signal.aborted) {
          cleanup();
          return;
        }

        active = true;
        try {
          navigator.vibrate?.(10);
        } catch {
          // Vibration is optional and may be blocked by the OS or browser.
        }
        document?.addEventListener('click', suppressClick, true);
      }, LONG_PRESS_DELAY);
    };

    element.addEventListener('touchstart', handleStart as EventListener, { passive: true });

    return () => {
      element.removeEventListener('touchstart', handleStart as EventListener);
      cleanup(true);
    };
  }
}
