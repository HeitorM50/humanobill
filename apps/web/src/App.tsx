import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { BrowserRouter, Route, Routes } from 'react-router'
import Layout from './components/Layout'
import Landing from './pages/Landing'
import Analyze from './pages/Analyze'
import Result from './pages/Result'
import Privacy from './pages/Privacy'

const queryClient = new QueryClient({ defaultOptions: { queries: { refetchOnWindowFocus: false, retry: false } } })

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <Routes>
          <Route element={<Layout />}>
            <Route index element={<Landing />} />
            <Route path="analisar" element={<Analyze />} />
            <Route path="r/:id" element={<Result />} />
            <Route path="privacidade" element={<Privacy />} />
            <Route path="*" element={<p className="text-lg">Página não encontrada.</p>} />
          </Route>
        </Routes>
      </BrowserRouter>
    </QueryClientProvider>
  )
}
