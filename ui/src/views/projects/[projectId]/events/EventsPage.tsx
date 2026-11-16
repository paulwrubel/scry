import { Alert } from 'flowbite-react';
import { DateTime } from 'luxon';
import { useNavigate, useParams, useSearchParams } from 'react-router-dom';
import { DEVICE_ZONE } from '@/contract/due';
import { useReplicaSnapshot } from '@/providers/Replica';
import type { Month } from '@/utils/events/calendar';
import {
  MONTH_PARAM,
  applyMonth,
  dayKey,
  monthGrid,
  monthKey,
  monthLabel,
  parseMonth,
  placeWeek,
  shiftMonth,
} from '@/utils/events/calendar';
import { MonthGrid } from './MonthGrid';
import { MonthHeader } from './MonthHeader';

export function EventsPage() {
  const { projectId } = useParams();
  const [searchParams, setSearchParams] = useSearchParams();
  const navigate = useNavigate();
  const { visible } = useReplicaSnapshot();

  const project = visible.projects.find((candidate) => candidate.id === projectId);
  const month = parseMonth(searchParams);
  const today = DateTime.now().setZone(DEVICE_ZONE);
  const weeks = monthGrid(month, DEVICE_ZONE);
  const events = project?.events ?? [];
  const lanes = weeks.map((week) => placeWeek(events, week, DEVICE_ZONE));
  const monthParams = `${MONTH_PARAM}=${monthKey(month)}`;

  if (project === undefined) {
    return (
      <div className="flex flex-1 items-center justify-center p-8">
        <Alert color="warning" className="max-w-md">
          Project not found. Choose another project from the navbar.
        </Alert>
      </div>
    );
  }

  function goToMonth(next: Month) {
    setSearchParams(applyMonth(searchParams, next), { replace: true });
  }

  return (
    <div className="flex min-h-0 w-full min-w-0 flex-1 flex-col">
      <MonthHeader
        label={monthLabel(month)}
        onPrevious={() => goToMonth(shiftMonth(month, -1))}
        onNext={() => goToMonth(shiftMonth(month, 1))}
        onToday={() => goToMonth({ year: today.year, month: today.month })}
        onCreate={() => navigate(`/projects/${project.id}/events/new?${monthParams}`)}
      />
      <MonthGrid
        weeks={weeks}
        lanes={lanes}
        month={month}
        today={today}
        projectId={project.id}
        monthQuery={`?${monthParams}`}
        onShowDay={(day) =>
          navigate(`/projects/${project.id}/events/days/${dayKey(day)}?${monthParams}`)
        }
      />
    </div>
  );
}
