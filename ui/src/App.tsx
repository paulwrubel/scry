import { createBrowserRouter, Navigate, RouterProvider } from 'react-router-dom';
import { Providers } from './layouts/Providers';
import { AuthenticatedRouteLayout } from './layouts/AuthenticatedRouteLayout';
import { Layout } from './layouts/Layout';
import { ProjectLayout } from './layouts/ProjectLayout';
import { ProjectRedirect } from './layouts/ProjectRedirect';
import { LoginPage } from './views/login';
import { ProjectsPage } from './views/projects';
import { SettingsPage } from './views/settings';

const routes = [
  {
    element: <Providers />,
    children: [
      {
        element: <Layout />,
        children: [
          { path: 'login', element: <LoginPage /> },
          {
            element: <AuthenticatedRouteLayout />,
            children: [
              { index: true, element: <ProjectRedirect /> },
              { path: 'projects', element: <ProjectsPage /> },
              {
                path: 'projects/:projectId',
                element: <ProjectLayout />,
                children: [{ path: 'tasks/:taskId' }],
              },
              { path: 'settings', element: <SettingsPage /> },
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
