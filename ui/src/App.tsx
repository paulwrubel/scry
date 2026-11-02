import { createBrowserRouter, RouterProvider } from 'react-router-dom';
import { Providers } from './layouts/Providers';
import { Layout } from './layouts/Layout';
import { HomePage } from './views';

const router = createBrowserRouter([
  {
    element: <Providers />,
    children: [
      {
        element: <Layout />,
        children: [{ path: '/', element: <HomePage /> }],
      },
    ],
  },
]);

export default function App() {
  return <RouterProvider router={router} />;
}
