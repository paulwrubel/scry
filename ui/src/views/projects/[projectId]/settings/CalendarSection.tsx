import { Button } from 'flowbite-react';
import { useRef, useState } from 'react';
import type { Event } from '@/generated/Event';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import { useAuth } from '@/providers/Auth';
import { useSync } from '@/providers/Sync';
import { exportEvents, getAPIURL, importEvents } from '@/utils/api';

async function readError(response: Response, fallback: string): Promise<string> {
  try {
    const body: unknown = await response.json();
    if (
      typeof body === 'object' &&
      body !== null &&
      'message' in body &&
      typeof body.message === 'string'
    ) {
      return body.message;
    }
  } catch {
    // error bodies are not guaranteed to be JSON
    return fallback;
  }
  return fallback;
}

function calendarFilename(projectName: string): string {
  const base = projectName.replace(/[^A-Za-z0-9._-]/g, '-');
  return base === '' ? 'calendar.ics' : `${base}.ics`;
}

export type CalendarSectionProps = {
  project: ProjectSnapshot;
};

export function CalendarSection(props: CalendarSectionProps) {
  const { project } = props;

  const { authenticatedFetch } = useAuth();
  const { syncNow } = useSync();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const fileInput = useRef<HTMLInputElement>(null);
  const baseUrl = getAPIURL();

  async function handleImport() {
    const file = fileInput.current?.files?.[0];
    if (file === undefined) {
      return;
    }

    setBusy(true);
    setError(null);
    setMessage(null);

    try {
      const ics = await file.text();
      const response = await importEvents(authenticatedFetch, baseUrl, project.id, ics);

      if (!response.ok) {
        setError(await readError(response, 'Could not import this calendar.'));
        return;
      }

      // the generated Event type mirrors the API response for a successful import
      const events = (await response.json()) as Event[];
      await syncNow();
      setMessage(`Imported ${events.length} event${events.length === 1 ? '' : 's'}.`);

      if (fileInput.current !== null) {
        // reset so selecting the same file again still fires a change event
        fileInput.current.value = '';
      }
    } catch {
      setError('Could not import this calendar.');
    } finally {
      setBusy(false);
    }
  }

  async function handleExport() {
    setBusy(true);
    setError(null);
    setMessage(null);

    try {
      const response = await exportEvents(authenticatedFetch, baseUrl, project.id);

      if (!response.ok) {
        setError(await readError(response, 'Could not export this calendar.'));
        return;
      }

      const calendar = await response.text();
      const objectUrl = URL.createObjectURL(new Blob([calendar], { type: 'text/calendar' }));
      const anchor = document.createElement('a');
      anchor.href = objectUrl;
      anchor.download = calendarFilename(project.name);
      anchor.click();
      anchor.remove();
      URL.revokeObjectURL(objectUrl);

      setMessage(`Exported ${project.name}.ics.`);
    } catch {
      setError('Could not export this calendar.');
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="flex flex-col gap-3">
      <input
        ref={fileInput}
        type="file"
        accept=".ics,text/calendar"
        aria-label="Calendar file"
        className="sr-only"
        onChange={() => void handleImport()}
      />
      <div className="flex flex-wrap gap-2">
        <Button color="alternative" disabled={busy} onClick={() => fileInput.current?.click()}>
          {busy ? 'Importing...' : 'Import calendar'}
        </Button>
        <Button color="light" disabled={busy} onClick={() => void handleExport()}>
          Export calendar
        </Button>
      </div>
      {error === null ? null : (
        <p className="text-danger text-sm" role="alert">
          {error}
        </p>
      )}
      {message === null ? null : <p className="text-muted text-sm">{message}</p>}
    </section>
  );
}
