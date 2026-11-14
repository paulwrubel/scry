import { createBrowserRouter, Navigate, RouterProvider } from 'react-router-dom';
import { Providers } from './layouts/Providers';
import { AuthenticatedRouteLayout } from './layouts/AuthenticatedRouteLayout';
import { authLoader } from './layouts/authLoader';
import { Layout } from './layouts/Layout';
import { ProjectLayout } from './layouts/ProjectLayout';
import { ProjectRedirect } from './layouts/ProjectRedirect';
import { ProjectTasksRedirect } from './layouts/ProjectTasksRedirect';
import { LoginPage } from './views/login/LoginPage';
import { ProjectsPage } from './views/projects/ProjectsPage';
import { ProjectSettingsPage } from './views/projects/[projectId]/settings/ProjectSettingsPage';

const routes = [
  {
    element: <Providers />,
    children: [
      {
        element: <Layout />,
        children: [
          { path: 'login', element: <LoginPage /> },
          {
            loader: authLoader,
            element: <AuthenticatedRouteLayout />,
            children: [
              { index: true, element: <ProjectRedirect /> },
              { path: 'projects', element: <ProjectsPage /> },
              { path: 'projects/:projectId', element: <ProjectTasksRedirect /> },
              {
                path: 'projects/:projectId/tasks',
                element: <ProjectLayout />,
                children: [{ path: ':taskId' }],
              },
              { path: 'projects/:projectId/settings', element: <ProjectSettingsPage /> },
              { path: '*', element: <Navigate to="/" replace /> },
            ],
          },
        ],
      },
    ],
  },
];

const router = createBrowserRouter(routes);

export default function App() {
  return <RouterProvider router={router} />;
}
