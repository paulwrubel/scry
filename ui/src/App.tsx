import { createBrowserRouter, Navigate, RouterProvider } from 'react-router-dom';
import { Providers } from './layouts/Providers';
import { Layout } from './layouts/Layout';
import { ProjectRedirect } from './layouts/ProjectRedirect';
import { BoardPage } from './views/projects/[projectId]';
import { LoginPage } from './views/login';
import { ProjectsPage } from './views/projects';
import { SettingsPage } from './views/settings';
import { TaskPage } from './views/projects/[projectId]/tasks/[taskId]';

const routes = [
  {
    element: <Providers />,
    children: [
      {
        element: <Layout />,
        children: [
          { index: true, element: <ProjectRedirect /> },
          { path: 'projects', element: <ProjectsPage /> },
          { path: 'projects/:projectId', element: <BoardPage /> },
          { path: 'projects/:projectId/tasks/:taskId', element: <TaskPage /> },
          { path: 'settings', element: <SettingsPage /> },
          { path: 'login', element: <LoginPage /> },
          { path: '*', element: <Navigate to="/" replace /> },
        ],
      },
    ],
  },
];

const router = createBrowserRouter(routes);

export default function App() {
  return <RouterProvider router={router} />;
}
