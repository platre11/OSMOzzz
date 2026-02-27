import { useState } from 'react'
import styled from 'styled-components'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { icons } from './lib/assets'
import StatusPage from './pages/StatusPage'
import SearchPage from './pages/SearchPage'
import RecentPage from './pages/RecentPage'
import ConfigPage from './pages/ConfigPage'

const queryClient = new QueryClient({
  defaultOptions: { queries: { refetchInterval: 10000 } },
})

type Page = 'status' | 'search' | 'recent' | 'config'

const NavBar = styled.nav`
  position: sticky;
  top: 0;
  z-index: 100;
  background: #fff;
  border-bottom: 1px solid #e8eaed;
  height: 56px;
  display: flex;
  align-items: center;
  padding: 0 32px;
  gap: 4px;
`

const Logo = styled.div`
  font-size: 17px;
  font-weight: 800;
  color: #5b5ef4;
  letter-spacing: -.03em;
  margin-right: 24px;

  span {
    font-size: 11px;
    font-weight: 400;
    color: #9ca3af;
    margin-left: 6px;
    letter-spacing: 0;
  }
`

const NavItem = styled.button<{ $active?: boolean }>`
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 14px;
  border-radius: 8px;
  font-size: 13px;
  font-weight: 500;
  border: none;
  cursor: pointer;
  transition: all .15s;
  background: ${({ $active }) => $active ? '#ededff' : 'transparent'};
  color: ${({ $active }) => $active ? '#5b5ef4' : '#6b7280'};

  &:hover {
    background: ${({ $active }) => $active ? '#ededff' : '#f3f4f6'};
    color: ${({ $active }) => $active ? '#5b5ef4' : '#1a1d23'};
  }
`

const PageWrapper = styled.main`
  max-width: 960px;
  margin: 0 auto;
  padding: 36px 32px;
`

const PAGES: { id: Page; label: string; Icon: React.ElementType }[] = [
  { id: 'status',  label: 'Statut',        Icon: icons.LayoutDashboard },
  { id: 'search',  label: 'Recherche',     Icon: icons.Search },
  { id: 'recent',  label: 'Récents',       Icon: icons.Clock },
  { id: 'config',  label: 'Configuration', Icon: icons.Settings },
]

export default function App() {
  const [page, setPage] = useState<Page>('status')

  return (
    <QueryClientProvider client={queryClient}>
      <NavBar>
        <Logo>OSMOzzz <span>local memory</span></Logo>
        {PAGES.map(({ id, label, Icon }) => (
          <NavItem key={id} $active={page === id} onClick={() => setPage(id)}>
            <Icon size={14} />
            {label}
          </NavItem>
        ))}
      </NavBar>
      <PageWrapper>
        {page === 'status'  && <StatusPage />}
        {page === 'search'  && <SearchPage />}
        {page === 'recent'  && <RecentPage />}
        {page === 'config'  && <ConfigPage />}
      </PageWrapper>
    </QueryClientProvider>
  )
}
