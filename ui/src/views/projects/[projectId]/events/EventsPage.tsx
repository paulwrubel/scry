import { Alert } from 'flowbite-react';
import { DateTime } from 'luxon';
import { useNavigate, useParams, useSearchParams } from 'react-router-dom';
import { DEVICE_ZONE } from '@/contract/due';
import { useReplicaSnapshot } from '@/providers/Replica';
import type { CalendarView, Month } from '@/utils/events/calendar';
import {
  WEEK_PARAM,
  applyCalendarView,
  applyMonth,
  applyWeek,
  dayKey,
  monthGrid,
  monthLabel,
  parseCalendarView,
  parseMonth,
  parseWeek,
  placeWeek,
  shiftMonth,
  weekDays,
  weekLabel,
} from '@/utils/events/calendar';
import { CalendarHeader } from './CalendarHeader';
import { MonthGrid } from './MonthGrid';
import { WeekGrid } from './WeekGrid';

export function EventsPage() {
  const { projectId } = useParams();
  const [searchParams, setSearchParams] = useSearchParams();
  const navigate = useNavigate();
  const { visible } = useReplicaSnapshot();

  const project = visible.projects.find((candidate) => candidate.id === projectId);
  const month = parseMonth(searchParams);
  const today = DateTime.now().setZone(DEVICE_ZONE);
  const view = parseCalendarView(searchParams);
  const week = weekDays(parseWeek(searchParams, DEVICE_ZONE, today));
  const weeks: DateTime[][] = view === 'month' ? monthGrid(month, DEVICE_ZONE) : [week];
  const events = project?.events ?? [];
  const lanes = view === 'month' ? weeks.map((item) => placeWeek(events, item, DEVICE_ZONE)) : [];
  const query = searchParams.toString();
  const querySuffix = query.length === 0 ? '' : `?${query}`;

  if (project === undefined) {
    return (
      <div className="flex flex-1 items-center justify-center p-8">
        <Alert color="warning" className="max-w-md">
          Project not found. Choose another project from the navbar.
        </Alert>
      </div>
    );
  }

  const activeProject = project;

  function goToMonth(next: Month) {
    setSearchParams(applyMonth(searchParams, next), { replace: true });
  }

  function goToWeek(next: DateTime) {
    setSearchParams(applyWeek(searchParams, next), { replace: true });
  }

  function goToView(next: CalendarView) {
    let params = applyCalendarView(searchParams, next);

    // the week view needs an anchor day, but only on the first switch; afterwards
    // the parameter that is already there must survive a toggle back and forth
    if (next === 'week' && !params.has(WEEK_PARAM)) {
      const inMonth = today.year === month.year && today.month === month.month;
      const anchor = inMonth
        ? today
        : DateTime.fromObject(
            { year: month.year, month: month.month, day: 1 },
            { zone: DEVICE_ZONE },
          );
      params = applyWeek(params, anchor);
    }

    if (next === 'month') {
      params = applyMonth(params, { year: week[0].year, month: week[0].month });
    }

    setSearchParams(params, { replace: true });
  }

  function showDay(day: DateTime) {
    navigate(`/projects/${activeProject.id}/events/days/${dayKey(day)}${querySuffix}`);
  }

  function createEventPath(): string {
    const params = new URLSearchParams(searchParams);
    params.delete('date');

    // a new event opened from the week view defaults to a day that is on screen
    if (view === 'week') {
      const target = week.some((day) => day.hasSame(today, 'day')) ? today : week[0];
      params.set('date', dayKey(target));
    }

    const suffix = params.toString();
    return `/projects/${activeProject.id}/events/new${suffix.length === 0 ? '' : `?${suffix}`}`;
  }

  return (
    <div className="flex min-h-0 w-full min-w-0 flex-1 flex-col">
      <CalendarHeader
        view={view}
        label={view === 'month' ? monthLabel(month) : weekLabel(week)}
        onPrevious={() => {
          if (view === 'month') {
            goToMonth(shiftMonth(month, -1));
          } else {
            goToWeek(week[0].minus({ days: 7 }));
          }
        }}
        onNext={() => {
          if (view === 'month') {
            goToMonth(shiftMonth(month, 1));
          } else {
            goToWeek(week[0].plus({ days: 7 }));
          }
        }}
        onToday={() => {
          if (view === 'month') {
            goToMonth({ year: today.year, month: today.month });
          } else {
            goToWeek(today);
          }
        }}
        onViewChange={goToView}
        onCreate={() => navigate(createEventPath())}
      />
      {view === 'month' ? (
        <MonthGrid
          weeks={weeks}
          lanes={lanes}
          month={month}
          today={today}
          projectId={activeProject.id}
          monthQuery={querySuffix}
          onShowDay={showDay}
        />
      ) : (
        <WeekGrid
          week={week}
          events={events}
          today={today}
          projectId={activeProject.id}
          query={query}
          onShowDay={showDay}
        />
      )}
    </div>
  );
}
