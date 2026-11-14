import { Alert } from 'flowbite-react';

export type ShellStateProps = {
  title: string;
  message: string;
  children?: React.ReactNode;
};

export function ShellState(props: ShellStateProps) {
  const { title, message, children } = props;

  return (
    <div className="flex min-h-dvh flex-col items-center justify-center gap-3 p-8 pt-[calc(env(safe-area-inset-top)+2rem)] pb-[calc(env(safe-area-inset-bottom)+2rem)]">
      <Alert color="info" className="max-w-md">
        <span className="font-medium">{title}</span>
        <span className="mt-1 block text-sm">{message}</span>
      </Alert>
      {children}
    </div>
  );
}
