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
    let pointerId: number | undefined;
    let latestEvent: TouchEvent | undefined;
    let active = false;
    let eventTarget: EventTarget | undefined;
    let ownerDocument: Document | undefined;
    let ownerWindow: Window | undefined;

    const suppressClick = (event: Event) => {
      event.preventDefault();
      event.stopImmediatePropagation();
    };

    const preventContextMenu = (event: Event) => {
      event.preventDefault();
    };

    const removeEventListeners = () => {
      if (eventTarget) {
        eventTarget.removeEventListener('touchmove', handleMove as EventListener, true);
        eventTarget.removeEventListener('touchend', handleEnd as EventListener, true);
        eventTarget.removeEventListener('touchcancel', handleCancel as EventListener, true);
        eventTarget.removeEventListener('pointerup', handlePointerEnd as EventListener, true);
        eventTarget.removeEventListener(
          'pointercancel',
          handlePointerCancel as EventListener,
          true,
        );
        eventTarget.removeEventListener('contextmenu', preventContextMenu, true);
      }

      if (ownerWindow) {
        ownerWindow.removeEventListener('blur', handleAbort);
        ownerWindow.removeEventListener('pagehide', handleAbort);
      }

      if (ownerDocument) {
        ownerDocument.removeEventListener('visibilitychange', handleVisibilityChange);
      }
    };

    const clearTimer = () => {
      if (timer !== undefined) {
        clearTimeout(timer);
        timer = undefined;
      }
    };

    const armClickGuard = () => {
      if (!ownerDocument) {
        return;
      }

      ownerDocument.addEventListener('click', suppressClick, true);
      if (clickGuardTimer !== undefined) {
        clearTimeout(clickGuardTimer);
      }
      clickGuardTimer = setTimeout(() => {
        ownerDocument?.removeEventListener('click', suppressClick, true);
        clickGuardTimer = undefined;
      }, 400);
    };

    const cleanup = (removeClickGuard = false) => {
      clearTimer();
      removeEventListeners();

      if (removeClickGuard && ownerDocument) {
        ownerDocument.removeEventListener('click', suppressClick, true);
        if (clickGuardTimer !== undefined) {
          clearTimeout(clickGuardTimer);
          clickGuardTimer = undefined;
        }
      }

      if (pointerId !== undefined) {
        try {
          element.releasePointerCapture(pointerId);
        } catch {
          // The pointer may already have been released.
        }
      }

      start = undefined;
      startTouchId = undefined;
      pointerId = undefined;
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

    const handleAbort = () => {
      if (active) {
        try {
          this.manager.actions.stop({ canceled: true });
        } finally {
          cleanup(true);
        }
        return;
      }

      cleanup(true);
    };

    const handleVisibilityChange = () => {
      if (ownerDocument?.visibilityState === 'hidden') {
        handleAbort();
      }
    };

    const handlePointerDown = (event: PointerEvent) => {
      if (event.pointerType === 'touch') {
        pointerId = event.pointerId;
      }
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
      ownerDocument = element.ownerDocument;
      ownerWindow = ownerDocument.defaultView ?? undefined;
      eventTarget = ownerWindow ?? ownerDocument;

      eventTarget.addEventListener('touchmove', handleMove as EventListener, {
        passive: false,
        capture: true,
      });
      eventTarget.addEventListener('touchend', handleEnd as EventListener, {
        passive: false,
        capture: true,
      });
      eventTarget.addEventListener('touchcancel', handleCancel as EventListener, {
        passive: false,
        capture: true,
      });
      eventTarget.addEventListener('pointerup', handlePointerEnd as EventListener, {
        capture: true,
      });
      eventTarget.addEventListener('pointercancel', handlePointerCancel as EventListener, {
        capture: true,
      });
      eventTarget.addEventListener('contextmenu', preventContextMenu, true);
      ownerWindow?.addEventListener('blur', handleAbort);
      ownerWindow?.addEventListener('pagehide', handleAbort);
      ownerDocument.addEventListener('visibilitychange', handleVisibilityChange);

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
          if (pointerId !== undefined) {
            element.setPointerCapture(pointerId);
          }
        } catch {
          // Pointer capture is only a safety net for end events.
        }
        try {
          navigator.vibrate?.(10);
        } catch {
          // Vibration is optional and may be blocked by the OS or browser.
        }
        ownerDocument?.addEventListener('click', suppressClick, true);
      }, LONG_PRESS_DELAY);
    };

    element.addEventListener('touchstart', handleStart as EventListener, { passive: true });
    element.addEventListener('pointerdown', handlePointerDown as EventListener, true);

    return () => {
      element.removeEventListener('touchstart', handleStart as EventListener);
      element.removeEventListener('pointerdown', handlePointerDown as EventListener, true);

      // React can reparent the sortable while a drag is active. Keep the
      // window-level end listeners alive until the gesture actually ends.
      if (!active) {
        cleanup(true);
      }
    };
  }
}
