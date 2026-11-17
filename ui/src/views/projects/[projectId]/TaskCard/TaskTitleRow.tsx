import { SiGithub } from 'react-icons/si';
import { VscGitPullRequest, VscIssues } from 'react-icons/vsc';
import { Link, useLocation } from 'react-router-dom';
import { GithubSyncStaleIcon } from '@/components/GithubSyncStaleIcon';
import { TitleText } from '@/components/TitleText';
import type { Task } from '@/generated/Task';
import { PRIORITY_DOT_CLASS } from '@/data/priority';
import { githubIconColor, githubKind, githubUrl } from '@/utils/github';

export type TaskTitleRowProps = {
  projectId: string;
  task: Task;
  showPriority: boolean;
};

export function TaskTitleRow(props: TaskTitleRowProps) {
  const { projectId, task, showPriority } = props;

  const { search } = useLocation();
  const closed = task.state === 'closed';
  const url = githubUrl(task);
  const kind = githubKind(task);
  const GitHubIcon = kind === 'pull' ? VscGitPullRequest : kind === 'issue' ? VscIssues : SiGithub;
  const githubLinkLabel =
    kind === 'pull'
      ? 'Open GitHub pull request'
      : kind === 'issue'
        ? 'Open GitHub issue'
        : 'Open GitHub item';

  return (
    <div className="flex min-w-0 items-start justify-between gap-2">
      <Link
        to={`/projects/${projectId}/tasks/${task.id}${search}`}
        draggable={false}
        className="flex min-w-0 flex-1 items-center gap-2"
      >
        <span
          className={`min-w-0 text-sm wrap-break-word hover:text-blue-400 hover:underline ${
            closed ? 'text-faint line-through' : ''
          }`}
        >
          <TitleText title={task.title} />
        </span>
      </Link>
      <div className="mt-1 flex shrink-0 items-center gap-1.5">
        <GithubSyncStaleIcon task={task} />
        {url === null ? null : (
          <a
            href={url}
            target="_blank"
            rel="noreferrer"
            draggable={false}
            aria-label={githubLinkLabel}
            title={githubLinkLabel}
            className={`${githubIconColor(task)} hover:text-zinc-200`}
            onClick={(event) => event.stopPropagation()}
          >
            <GitHubIcon aria-hidden="true" className="h-3.5 w-3.5" />
          </a>
        )}
        {showPriority && (task.priority === 'critical' || task.priority === 'elevated') ? (
          <span
            role="img"
            aria-label={`${task.priority} priority`}
            title={task.priority}
            className={`h-2 w-2 shrink-0 rounded-full ${PRIORITY_DOT_CLASS[task.priority]}`}
          />
        ) : null}
      </div>
    </div>
  );
}
