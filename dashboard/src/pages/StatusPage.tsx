import { useQuery, useQueryClient, useMutation } from '@tanstack/react-query'
import styled, { keyframes } from 'styled-components'
import { api } from '../api'
import type { StatusData } from '../api'
import { icons } from '../lib/assets'
import RecentPage from './RecentPage'

const SOURCE_META: Record<string, { label: string; color: string; bg: string; Icon: React.ElementType }> = {
  email:    { label: 'Gmail',      color: '#dc2626', bg: '#fef2f2', Icon: icons.Mail },
  chrome:   { label: 'Chrome',     color: '#1d4ed8', bg: '#eff6ff', Icon: icons.Chrome },
  file:     { label: 'Fichiers',   color: '#16a34a', bg: '#f0fdf4', Icon: icons.FileText },
  imessage: { label: 'iMessage',   color: '#9333ea', bg: '#faf5ff', Icon: icons.MessageCircle },
  safari:   { label: 'Safari',     color: '#ea580c', bg: '#fff7ed', Icon: icons.Globe },
  notes:    { label: 'Notes',      color: '#ca8a04', bg: '#fefce8', Icon: icons.BookOpen },
  terminal: { label: 'Terminal',   color: '#475569', bg: '#f8fafc', Icon: icons.Terminal },
  calendar: { label: 'Calendrier', color: '#0d9488', bg: '#f0fdfa', Icon: icons.Calendar },
}

const SOURCE_MAX: Record<string, number | null> = {
  email:    5000,
  chrome:   10000,
  safari:   10000,
  imessage: 5000,
  terminal: 5000,
  notes:    2000,
  calendar: null,
  file:     null,
}

const spin = keyframes`to { transform: rotate(360deg); }`

const Page = styled.div`
  display: flex;
  flex-direction: column;
  gap: 24px;
`

const PageHeader = styled.div``

const PageTitle = styled.h1`
  font-size: 22px;
  font-weight: 700;
  color: #1a1d23;
  letter-spacing: -.02em;
`

const PageSubtitle = styled.p`
  margin-top: 6px;
  font-size: 13px;
  color: #6b7280;
  display: flex;
  align-items: center;
  gap: 6px;
`

const StatusDot = styled.span<{ $active?: boolean }>`
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: ${({ $active }) => $active ? '#10b981' : '#ef4444'};
  display: inline-block;
`

const Grid = styled.div`
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 16px;
`

const Card = styled.div`
  background: #fff;
  border: 1px solid #e8eaed;
  border-radius: 14px;
  padding: 20px;
  box-shadow: 0 1px 3px rgba(0,0,0,.05);
  transition: box-shadow .15s;

  &:hover { box-shadow: 0 4px 16px rgba(0,0,0,.08); }
`

const CardIcon = styled.div<{ $bg: string; $color: string }>`
  width: 36px;
  height: 36px;
  border-radius: 10px;
  background: ${({ $bg }) => $bg};
  color: ${({ $color }) => $color};
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 12px;
`

const CardLabel = styled.p`
  font-size: 12px;
  font-weight: 600;
  color: #9ca3af;
  text-transform: uppercase;
  letter-spacing: .05em;
`

const CardCount = styled.p`
  font-size: 28px;
  font-weight: 700;
  color: #1a1d23;
  letter-spacing: -.03em;
  line-height: 1.1;
  margin-top: 2px;
`

const CardSub = styled.p`
  font-size: 11px;
  color: #9ca3af;
  margin-top: 2px;
`

const CardMax = styled.p`
  font-size: 11px;
  color: #9ca3af;
  margin-top: 6px;
`

const MiniBar = styled.div<{ $pct: number; $color: string }>`
  height: 3px;
  border-radius: 2px;
  background: #f3f4f6;
  margin-top: 4px;
  overflow: hidden;
  &::after {
    content: '';
    display: block;
    height: 100%;
    width: ${({ $pct }) => Math.min($pct, 100)}%;
    background: ${({ $color }) => $color};
    border-radius: 2px;
    transition: width .4s ease;
  }
`

const ErrorTag = styled.div`
  margin-top: 8px;
  font-size: 11px;
  color: #dc2626;
  background: #fef2f2;
  border-radius: 6px;
  padding: 4px 8px;
`

const DiskAccessBtn = styled.button`
  margin-top: 10px;
  width: 100%;
  padding: 7px 10px;
  border-radius: 8px;
  border: 1px solid #fbbf24;
  background: #fffbeb;
  color: #92400e;
  font-size: 11px;
  font-weight: 600;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 5px;
  transition: background .15s;
  &:hover { background: #fef3c7; }
`

const ReindexBtn = styled.button<{ $loading?: boolean; $done?: boolean }>`
  margin-top: 10px;
  width: 100%;
  padding: 7px 10px;
  border-radius: 8px;
  border: 1px solid ${({ $done }) => $done ? '#bbf7d0' : '#e5e7eb'};
  background: ${({ $loading, $done }) => $loading ? '#f3f4f6' : $done ? '#f0fdf4' : '#fff'};
  color: ${({ $loading, $done }) => $loading ? '#9ca3af' : $done ? '#16a34a' : '#374151'};
  font-size: 11px;
  font-weight: 600;
  cursor: ${({ $loading }) => $loading ? 'not-allowed' : 'pointer'};
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 5px;
  transition: all .15s;
  &:hover { background: ${({ $loading, $done }) => $loading ? '#f3f4f6' : $done ? '#dcfce7' : '#f9fafb'}; }
`

// Sources qui nécessitent l'accès disque complet sur macOS
const DISK_ACCESS_SOURCES = new Set(['imessage', 'safari', 'notes', 'calendar'])

const Loader = styled.div`
  width: 20px;
  height: 20px;
  border: 2px solid #e5e7eb;
  border-top-color: #5b5ef4;
  border-radius: 50%;
  animation: ${spin} .7s linear infinite;
  margin: 60px auto;
`

const ErrorMsg = styled.p`
  text-align: center;
  padding: 60px;
  color: #9ca3af;
`


export default function StatusPage() {
  const queryClient = useQueryClient()
  const { data, isLoading, error } = useQuery<StatusData>({
    queryKey: ['status'],
    queryFn: api.getStatus,
  })

  const reindexMutation = useMutation({
    mutationFn: api.reindexImessage,
    onSuccess: () => {
      setTimeout(() => queryClient.invalidateQueries({ queryKey: ['status'] }), 1000)
    },
  })

  if (isLoading) return <Loader />
  if (error)     return <ErrorMsg>Impossible de joindre le daemon OSMOzzz.</ErrorMsg>
  if (!data)     return null

  const total = Object.values(data.sources).reduce((s, v) => s + v.count, 0)

  const diskSourcesToCheck = Object.entries(data.sources)
    .filter(([src]) => DISK_ACCESS_SOURCES.has(src))
  const needsDiskAccess = diskSourcesToCheck.length > 0
    && diskSourcesToCheck.every(([, v]) => v.count === 0)

  function openPrivacySettings() {
    fetch('/api/open?url=' + encodeURIComponent('x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles'))
  }

  return (
    <Page>
      <PageHeader>
        <PageTitle>Vue d'ensemble</PageTitle>
        <PageSubtitle>
          <StatusDot $active={data.daemon_status === 'running'} />
          Daemon {data.daemon_status === 'running' ? 'actif' : 'inactif'}
          {' · '}
          {total.toLocaleString('fr-FR')} documents indexés
        </PageSubtitle>
      </PageHeader>

      <Grid>
        {[
          'email','chrome','file','imessage','safari','notes','terminal','calendar',
        ].filter(s => s in data.sources).map(source => {
          const status = data.sources[source]
          const meta = SOURCE_META[source] ?? { label: source, color: '#374151', bg: '#f3f4f6', Icon: icons.Database }
          const { Icon } = meta
          const max = SOURCE_MAX[source] ?? null
          const pct = max ? (status.count / max) * 100 : 0
          return (
            <Card key={source}>
              <CardIcon $bg={meta.bg} $color={meta.color}>
                <Icon size={16} />
              </CardIcon>
              <CardLabel>{meta.label}</CardLabel>
              <CardCount>{status.count.toLocaleString('fr-FR')}</CardCount>
              <CardSub>documents indexés</CardSub>
              {max !== null && (
                <>
                  <MiniBar $pct={pct} $color={meta.color} />
                  <CardMax>{status.count.toLocaleString('fr-FR')} / {max.toLocaleString('fr-FR')}</CardMax>
                </>
              )}
              {status.error && <ErrorTag>⚠ {status.error}</ErrorTag>}
              {needsDiskAccess && DISK_ACCESS_SOURCES.has(source) && (
                <DiskAccessBtn onClick={openPrivacySettings}>
                  🔒 Autoriser l'accès disque →
                </DiskAccessBtn>
              )}
              {source === 'imessage' && (
                <ReindexBtn
                  $loading={reindexMutation.isPending}
                  $done={reindexMutation.isSuccess}
                  disabled={reindexMutation.isPending}
                  onClick={() => reindexMutation.mutate()}
                >
                  {reindexMutation.isPending
                    ? '⏳ Indexation en cours...'
                    : reindexMutation.isSuccess
                    ? `✓ ${reindexMutation.data}`
                    : '↺ Re-indexer'}
                </ReindexBtn>
              )}
            </Card>
          )
        })}
      </Grid>

      <RecentPage />
    </Page>
  )
}
