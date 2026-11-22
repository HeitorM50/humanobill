import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { BrowserRouter, Route, Routes } from 'react-router'
import Layout from './components/Layout'
import { Page } from './components/ui'
import Landing from './pages/Landing'
import Analyze from './pages/Analyze'
import Result from './pages/Result'
import Letter from './pages/Letter'
import Compare from './pages/Compare'
import Guided from './pages/Guided'
import Privacy from './pages/Privacy'
import CancelReminder from './pages/CancelReminder'

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
            <Route path="r/:id/carta" element={<Letter />} />
            <Route path="guiado" element={<Guided />} />
            <Route path="guiado/:id" element={<Guided />} />
            <Route path="comparar" element={<Compare />} />
            <Route path="lembrete/cancelar/:token" element={<CancelReminder />} />
            <Route path="privacidade" element={<Privacy />} />
            <Route path="*" element={<Page><h1 className="text-h2">Página não encontrada.</h1></Page>} />
          </Route>
        </Routes>
      </BrowserRouter>
    </QueryClientProvider>
  )
}
