import { useState, useEffect } from 'react'
import styled from 'styled-components'
import { QueryClient, QueryClientProvider, useQuery } from '@tanstack/react-query'
import { icons } from './lib/assets'
import StatusPage from './pages/StatusPage'
import ConfigPage from './pages/ConfigPage'
import NetworkPage from './pages/NetworkPage'
import ActionsPage from './pages/ActionsPage'
import { api } from './api'

const queryClient = new QueryClient({
  defaultOptions: { queries: { refetchInterval: 10000 } },
})

type Page = 'status' | 'config' | 'network' | 'actions'

const SIDEBAR_W = 260

// ─── Top bar ──────────────────────────────────────────────────────────────────

const TopBar = styled.header`
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  height: 56px;
  background: #fff;
  border-bottom: 1px solid #e8eaed;
  display: flex;
  align-items: center;
  padding: 0 24px;
  z-index: 200;
`

const Logo = styled.div`
  font-size: 17px;
  font-weight: 800;
  color: #5b5ef4;
  letter-spacing: -.03em;

  span {
    font-size: 11px;
    font-weight: 400;
    color: #9ca3af;
    margin-left: 6px;
    letter-spacing: 0;
  }
`

// ─── Sidebar fixe à gauche ────────────────────────────────────────────────────

const Sidebar = styled.nav`
  position: fixed;
  top: 56px;
  left: 0;
  bottom: 0;
  width: ${SIDEBAR_W}px;
  background: #fff;
  border-right: 1px solid #e8eaed;
  display: flex;
  flex-direction: column;
  padding: 16px 12px;
  gap: 2px;
  z-index: 100;
`

// ─── Main layout ─────────────────────────────────────────────────────────────

const Layout = styled.div`
  margin-left: ${SIDEBAR_W}px;
  margin-top: 56px;
  padding: 36px 40px;
`

const ContentInner = styled.div`
  max-width: 1200px;
  margin: 0 auto;
`

const NavBadge = styled.span`
  display: inline-flex; align-items: center; justify-content: center;
  min-width: 17px; height: 17px; padding: 0 4px;
  background: #ef4444; color: #fff; border-radius: 99px;
  font-size: 10px; font-weight: 700; margin-left: auto;
`

const ScrollTopBtn = styled.button<{ $visible: boolean }>`
  position: fixed;
  bottom: 32px;
  right: 32px;
  width: 44px;
  height: 44px;
  border-radius: 50%;
  background: #5b5ef4;
  color: #fff;
  border: none;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 4px 12px rgba(91,94,244,.35);
  z-index: 999;
  opacity: ${({ $visible }) => $visible ? 1 : 0};
  pointer-events: ${({ $visible }) => $visible ? 'auto' : 'none'};
  transition: opacity .2s;
  &:hover { background: #4a4dd4; }
`

const NavItem = styled.button<{ $active?: boolean }>`
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border-radius: 8px;
  font-size: 13px;
  font-weight: 500;
  border: none;
  cursor: pointer;
  width: 100%;
  text-align: left;
  transition: all .15s;
  background: ${({ $active }) => $active ? '#ededff' : 'transparent'};
  color: ${({ $active }) => $active ? '#5b5ef4' : '#6b7280'};

  &:hover {
    background: ${({ $active }) => $active ? '#ededff' : '#f3f4f6'};
    color: ${({ $active }) => $active ? '#5b5ef4' : '#1a1d23'};
  }
`


// ─── Pages ────────────────────────────────────────────────────────────────────

const PAGES: { id: Page; label: string; Icon: React.ElementType }[] = [
  { id: 'status',  label: 'Dashboard',   Icon: icons.LayoutDashboard },
  { id: 'actions', label: 'Actions MCP', Icon: icons.Zap },
  { id: 'network', label: 'Réseau',      Icon: icons.Network },
  { id: 'config',  label: 'Connecteurs', Icon: icons.Settings },
]

const VALID_PAGES: Page[] = ['status', 'actions', 'network', 'config']

function pageFromHash(): Page {
  const hash = window.location.hash.slice(1) as Page
  return VALID_PAGES.includes(hash) ? hash : 'status'
}

function AppInner() {
  const [page, setPage] = useState<Page>(pageFromHash)
  const [showScrollTop, setShowScrollTop] = useState(false)

  useEffect(() => {
    window.location.hash = page
    window.scrollTo(0, 0)
  }, [page])

  useEffect(() => {
    const onScroll = () => setShowScrollTop(window.scrollY >= window.innerHeight)
    window.addEventListener('scroll', onScroll, { passive: true })
    return () => window.removeEventListener('scroll', onScroll)
  }, [])

  // Sync si l'utilisateur navigue avec les boutons précédent/suivant du browser
  useEffect(() => {
    const onHashChange = () => setPage(pageFromHash())
    window.addEventListener('hashchange', onHashChange)
    return () => window.removeEventListener('hashchange', onHashChange)
  }, [])

  const { data: pending = [] } = useQuery({
    queryKey: ['actions-pending'],
    queryFn: api.getActionsPending,
    refetchInterval: 10_000,
  })

  return (
    <>
      <TopBar>
        <Logo>OSMOzzz <span>local memory</span></Logo>
      </TopBar>
      <Sidebar>
        {PAGES.map(({ id, label, Icon }) => (
          <NavItem key={id} $active={page === id} onClick={() => setPage(id)}>
            <Icon size={14} />
            {label}
            {id === 'actions' && pending.length > 0 && (
              <NavBadge>{pending.length}</NavBadge>
            )}
          </NavItem>
        ))}
      </Sidebar>
      <Layout>
        <ContentInner>
          {page === 'status'  && <StatusPage />}
          {page === 'actions' && <ActionsPage />}
          {page === 'network' && <NetworkPage />}
          {page === 'config'  && <ConfigPage />}
        </ContentInner>
      </Layout>
      <ScrollTopBtn $visible={showScrollTop} onClick={() => window.scrollTo({ top: 0, behavior: 'smooth' })}>
        <icons.ChevronUp size={18} />
      </ScrollTopBtn>
    </>
  )
}

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <AppInner />
    </QueryClientProvider>
  )
}
