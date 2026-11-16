import type { DrawerTheme } from 'flowbite-react';
import type { DeepPartial } from 'flowbite-react/types';

const mobileSheetClasses =
  'inset-x-0 bottom-0 h-[75dvh] w-full rounded-t-xl border-t pb-[env(safe-area-inset-bottom)]';
const desktopDrawerClasses =
  'md:inset-y-4 md:right-4 md:left-auto md:h-auto md:w-[54rem] md:rounded-xl md:border md:pb-0 md:shadow-2xl';

/// The detail drawer shared by the task board and the event calendar: a bottom
/// sheet on small screens and a right-hand panel on large ones.
export const drawerTheme: DeepPartial<DrawerTheme> = {
  root: {
    base: 'fixed z-40 overflow-y-auto border-line bg-surface p-0 transition-transform duration-300 ease-out',
    // Flowbite accepts one Drawer position per render, so the bottom sheet
    // restores the desktop right-drawer geometry at the md breakpoint.
    position: {
      bottom: {
        on: `${mobileSheetClasses} ${desktopDrawerClasses}`,
        off: `${mobileSheetClasses} ${desktopDrawerClasses} translate-y-full md:translate-y-0 md:translate-x-[calc(100%_+_1rem)]`,
      },
    },
  },
};
