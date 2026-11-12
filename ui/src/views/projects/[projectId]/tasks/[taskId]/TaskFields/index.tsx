import type { Task } from '@/generated/Task';
import { DescriptionField } from './DescriptionField';
import { TagsField } from './TagsField';

export type TaskFieldsProps = {
  task: Task;
  onSaveDescription: (description: string) => Promise<void>;
  onSaveTags: (tags: string[]) => Promise<void>;
};

export function TaskFields(props: TaskFieldsProps) {
  const { task, onSaveDescription, onSaveTags } = props;

  const descriptionReadOnly = (task.external_metadata?.description ?? null) !== null;

  return (
    <section className="flex flex-col gap-4">
      <DescriptionField task={task} readOnly={descriptionReadOnly} onSave={onSaveDescription} />
      <TagsField task={task} onSave={onSaveTags} />
    </section>
  );
}
