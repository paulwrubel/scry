import { Button } from 'flowbite-react';

export function LoginPage() {
  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-3 p-8 text-center">
      <h1 className="text-2xl font-semibold">Sign in</h1>
      <p className="text-sm text-zinc-400">Browser sign-in is not available in this build.</p>
      <Button color="default" disabled>
        Sign in with API token
      </Button>
    </div>
  );
}
