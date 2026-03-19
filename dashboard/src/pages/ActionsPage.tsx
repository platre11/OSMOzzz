import { useState, useEffect, useRef } from 'react'
import styled, { keyframes } from 'styled-components'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '../api'
import type { ActionRequest, ActionEvent } from '../api'
import { PrivacyPanel } from '../components/PrivacyPanel'
import BlacklistPanel from '../components/BlacklistPanel'

// ─── Styles ───────────────────────────────────────────────────────────────────

const spin = keyframes`to { transform: rotate(360deg); }`

const Page = styled.div`display: flex; flex-direction: column; gap: 32px;`

const PageTitle = styled.h1`font-size: 22px; font-weight: 700; color: #1a1d23; letter-spacing: -.02em;`

const SectionLabel = styled.h2`
  font-size: 11px; font-weight: 600; color: #9ca3af;
  text-transform: uppercase; letter-spacing: .08em;
  margin-bottom: -16px;
`

// ── Autorisations MCP ────────────────────────────────────────────────────────

const PermSection = styled.div`
  background: #fff; border: 1px solid #e8eaed; border-radius: 14px;
  overflow: hidden; box-shadow: 0 1px 3px rgba(0,0,0,.05);
`

const PermHeader = styled.div`
  padding: 18px 20px 14px; border-bottom: 1px solid #f3f4f6;
`

const PermTitle = styled.h2`font-size: 14px; font-weight: 600; color: #1a1d23; margin: 0;`

const PermDesc = styled.p`font-size: 12px; color: #6b7280; margin: 4px 0 0; line-height: 1.5;`

const PermRow = styled.div`
  display: flex; align-items: center; justify-content: space-between;
  padding: 14px 20px; border-bottom: 1px solid #f9fafb;
  &:last-child { border-bottom: none; }
`

const PermLabel = styled.span`font-size: 13px; font-weight: 500; color: #1a1d23;`

const PermHint = styled.span`font-size: 11px; color: #9ca3af; display: block; margin-top: 2px;`

const Toggle = styled.button<{ $on: boolean }>`
  width: 40px; height: 22px; border-radius: 11px; border: none; cursor: pointer;
  background: ${({ $on }) => $on ? '#5b5ef4' : '#d1d5db'};
  position: relative; transition: background .2s; flex-shrink: 0;
  &::after {
    content: ''; position: absolute; width: 16px; height: 16px;
    border-radius: 50%; background: white; top: 3px;
    left: ${({ $on }) => $on ? '21px' : '3px'}; transition: left .2s;
    box-shadow: 0 1px 3px rgba(0,0,0,.2);
  }
`

// ── Liste noire ──────────────────────────────────────────────────────────────

const BlacklistCard = styled.div`
  background: #fff; border: 1px solid #e8eaed; border-radius: 14px;
  padding: 18px 20px; box-shadow: 0 1px 3px rgba(0,0,0,.05);
  display: flex; align-items: center; justify-content: space-between;
`

const BlacklistLeft = styled.div``

const BlacklistTitle = styled.p`font-size: 14px; font-weight: 600; color: #1a1d23;`

const BlacklistDesc = styled.p`font-size: 12px; color: #6b7280; margin-top: 3px;`

const BlacklistCount = styled.span<{ $n: number }>`
  font-size: 11px; font-weight: 600; padding: 3px 10px; border-radius: 20px;
  background: ${({ $n }) => $n > 0 ? '#fee2e2' : '#f3f4f6'};
  color: ${({ $n }) => $n > 0 ? '#991b1b' : '#6b7280'};
  margin-right: 10px;
`

const ManageBtn = styled.button`
  padding: 8px 16px; border-radius: 9px; font-size: 13px; font-weight: 500;
  border: 1px solid #e5e7eb; background: #fff; color: #374151; cursor: pointer;
  transition: all .15s;
  &:hover { background: #f3f4f6; border-color: #d1d5db; }
`

// ── Actions (tabs + cards) ───────────────────────────────────────────────────

const ActionsBlock = styled.div`display: flex; flex-direction: column; gap: 12px;`

const ActionsHeader = styled.div`
  display: flex; align-items: center; justify-content: space-between;
`

const TabRow = styled.div`display: flex; gap: 6px;`

const Tab = styled.button<{ $active?: boolean }>`
  padding: 6px 14px; border-radius: 8px; font-size: 12px; font-weight: 500;
  border: 1px solid ${({ $active }) => $active ? '#5b5ef4' : '#e5e7eb'};
  background: ${({ $active }) => $active ? '#5b5ef4' : '#fff'};
  color: ${({ $active }) => $active ? '#fff' : '#6b7280'};
  cursor: pointer; transition: all .15s;
  &:hover { background: ${({ $active }) => $active ? '#4a4de3' : '#f3f4f6'}; }
`

const BadgeCount = styled.span`
  display: inline-flex; align-items: center; justify-content: center;
  min-width: 18px; height: 18px; padding: 0 5px;
  background: #ef4444; color: #fff; border-radius: 99px;
  font-size: 10px; font-weight: 700; margin-left: 6px;
`

const EmptyMsg = styled.p`text-align: center; padding: 60px; color: #9ca3af;`

const Loader = styled.div`
  width: 20px; height: 20px; border: 2px solid #e5e7eb; border-top-color: #5b5ef4;
  border-radius: 50%; animation: ${spin} .7s linear infinite; margin: 60px auto;
`

const CardList = styled.div`display: flex; flex-direction: column; gap: 12px;`

const ActionCard = styled.div<{ $status: string }>`
  background: #fff;
  border: 1px solid ${({ $status }) =>
    $status === 'pending'  ? '#fbbf24' :
    $status === 'approved' ? '#10b981' :
    $status === 'rejected' ? '#ef4444' :
    '#e5e7eb'};
  border-radius: 14px; padding: 20px 24px;
  box-shadow: 0 1px 3px rgba(0,0,0,.05);
`

const CardTop = styled.div`display: flex; align-items: center; justify-content: space-between; margin-bottom: 12px;`

const ToolBadge = styled.span`
  font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: .06em;
  padding: 3px 10px; border-radius: 6px; background: #ededff; color: #5b5ef4;
`

const StatusPill = styled.span<{ $status: string }>`
  font-size: 11px; font-weight: 600; padding: 3px 10px; border-radius: 99px;
  background: ${({ $status }) =>
    $status === 'pending'  ? '#fef3c7' :
    $status === 'approved' ? '#d1fae5' :
    $status === 'rejected' ? '#fee2e2' :
    '#f3f4f6'};
  color: ${({ $status }) =>
    $status === 'pending'  ? '#92400e' :
    $status === 'approved' ? '#065f46' :
    $status === 'rejected' ? '#991b1b' :
    '#6b7280'};
`

const StatusLabel: Record<string, string> = {
  pending:  'En attente',
  approved: 'Approuvée',
  rejected: 'Refusée',
  expired:  'Expirée',
}

const PreviewBox = styled.pre`
  background: #f8fafc; border: 1px solid #e8eaed; border-radius: 10px;
  padding: 14px 16px; font-size: 12px; color: #374151; line-height: 1.6;
  white-space: pre-wrap; word-break: break-word; margin: 0; font-family: inherit;
`

const CardDate = styled.span`font-size: 11px; color: #9ca3af;`

const ActionBtns = styled.div`display: flex; gap: 10px; margin-top: 16px;`

const ApproveBtn = styled.button`
  flex: 1; padding: 10px; border-radius: 10px; font-size: 13px; font-weight: 600;
  border: none; background: #10b981; color: #fff; cursor: pointer; transition: background .15s;
  &:hover { background: #059669; }
  &:disabled { opacity: .5; cursor: not-allowed; }
`

const RejectBtn = styled.button`
  flex: 1; padding: 10px; border-radius: 10px; font-size: 13px; font-weight: 600;
  border: 1px solid #fca5a5; background: #fef2f2; color: #dc2626; cursor: pointer; transition: all .15s;
  &:hover { background: #fee2e2; }
  &:disabled { opacity: .5; cursor: not-allowed; }
`

const LiveDot = styled.span<{ $active: boolean }>`
  display: inline-block; width: 7px; height: 7px; border-radius: 50%;
  background: ${({ $active }) => $active ? '#10b981' : '#d1d5db'};
  margin-right: 6px;
`

const SseStatus = styled.p`font-size: 11px; color: #9ca3af; display: flex; align-items: center;`

const ExecResult = styled.div<{ $ok: boolean }>`
  margin-top: 12px; padding: 10px 14px; border-radius: 8px; font-size: 12px; font-weight: 500;
  background: ${({ $ok }) => $ok ? '#d1fae5' : '#fee2e2'};
  color: ${({ $ok }) => $ok ? '#065f46' : '#991b1b'};
`

// ─── Composant carte action ───────────────────────────────────────────────────

function ActionCardItem({ action, onDecision }: {
  action: ActionRequest
  onDecision: () => void
}) {
  const [loading, setLoading] = useState(false)

  async function approve() {
    setLoading(true)
    try { await api.approveAction(action.id) } finally { setLoading(false); onDecision() }
  }

  async function reject() {
    setLoading(true)
    try { await api.rejectAction(action.id) } finally { setLoading(false); onDecision() }
  }

  const date = new Date(action.created_at * 1000).toLocaleString('fr-FR')
  const expiresIn = Math.max(0, action.expires_at - Math.floor(Date.now() / 1000))
  const toolLabel = action.tool.replace('act_', '').replace(/_/g, ' ')

  return (
    <ActionCard $status={action.status}>
      <CardTop>
        <ToolBadge>{toolLabel}</ToolBadge>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
          {action.status === 'pending' && expiresIn > 0 && (
            <CardDate>expire dans {Math.ceil(expiresIn / 60)} min</CardDate>
          )}
          <StatusPill $status={action.status}>{StatusLabel[action.status] ?? action.status}</StatusPill>
        </div>
      </CardTop>

      <PreviewBox>{action.preview}</PreviewBox>

      <div style={{ display: 'flex', justifyContent: 'space-between', marginTop: 10 }}>
        <CardDate>Demandée le {date}</CardDate>
      </div>

      {action.execution_result && (
        <ExecResult $ok={action.execution_result.startsWith('ok:')}>
          {action.execution_result.startsWith('ok:') ? '✓ ' : '✕ '}
          {action.execution_result.replace(/^(ok|err): /, '')}
        </ExecResult>
      )}

      {action.status === 'pending' && (
        <ActionBtns>
          <ApproveBtn onClick={approve} disabled={loading}>✓ Approuver</ApproveBtn>
          <RejectBtn onClick={reject} disabled={loading}>✕ Rejeter</RejectBtn>
        </ActionBtns>
      )}
    </ActionCard>
  )
}

// ─── Page principale ──────────────────────────────────────────────────────────

export default function ActionsPage() {
  const [tab, setTab] = useState<'pending' | 'history'>('pending')
  const [sseConnected, setSseConnected] = useState(false)
  const [showBlacklist, setShowBlacklist] = useState(false)
  const queryClient = useQueryClient()
  const esRef = useRef<EventSource | null>(null)

  // ── Permissions MCP ─────────────────────────────────────────────────────
  const { data: permsData } = useQuery({
    queryKey: ['permissions'],
    queryFn:  api.getPermissions,
  })
  const [permNotion, setPermNotion] = useState(false)
  const [permGithub, setPermGithub] = useState(false)
  const [permLinear, setPermLinear] = useState(false)
  const [permJira,   setPermJira]   = useState(false)

  useEffect(() => {
    if (permsData) {
      setPermNotion(permsData.notion ?? false)
      setPermGithub(permsData.github ?? false)
      setPermLinear(permsData.linear ?? false)
      setPermJira(permsData.jira ?? false)
    }
  }, [permsData])

  function togglePerm(
    key: 'notion' | 'github' | 'linear' | 'jira',
    current: boolean,
    setter: (v: boolean) => void,
  ) {
    const next = !current
    setter(next)
    api.savePermissions({
      notion: key === 'notion' ? next : permNotion,
      github: key === 'github' ? next : permGithub,
      linear: key === 'linear' ? next : permLinear,
      jira:   key === 'jira'   ? next : permJira,
    }).then(() => queryClient.invalidateQueries({ queryKey: ['permissions'] }))
  }

  // ── Accès sources MCP ───────────────────────────────────────────────────
  type SourceKey = 'email'|'imessage'|'chrome'|'safari'|'notes'|'calendar'|'terminal'|'file'|'notion'|'github'|'linear'|'jira'
  const { data: sourceData } = useQuery({ queryKey: ['source-access'], queryFn: api.getSourceAccess })
  const defaultSources = { email:true,imessage:true,chrome:true,safari:true,notes:true,calendar:true,terminal:true,file:true,notion:true,github:true,linear:true,jira:true }
  const [sources, setSources] = useState(defaultSources)
  useEffect(() => { if (sourceData) setSources(sourceData) }, [sourceData])

  function toggleSource(key: SourceKey) {
    const next = { ...sources, [key]: !sources[key] }
    setSources(next)
    api.saveSourceAccess(next).then(() => queryClient.invalidateQueries({ queryKey: ['source-access'] }))
  }

  // ── Blacklist count ──────────────────────────────────────────────────────
  const { data: blacklistData } = useQuery({
    queryKey: ['blacklist'],
    queryFn:  api.getBlacklist,
  })
  const bannedCount = blacklistData?.entries.length ?? 0

  // ── SSE pour mises à jour temps réel ────────────────────────────────────
  useEffect(() => {
    const es = new EventSource('/api/actions/stream')
    esRef.current = es
    es.onopen  = () => setSseConnected(true)
    es.onerror = () => setSseConnected(false)
    es.onmessage = (e) => {
      try {
        const event = JSON.parse(e.data) as ActionEvent
        if (event.kind === 'new' || event.kind === 'updated') {
          queryClient.invalidateQueries({ queryKey: ['actions-pending'] })
          queryClient.invalidateQueries({ queryKey: ['actions-all'] })
        }
      } catch { /* ignore */ }
    }
    return () => { es.close(); setSseConnected(false) }
  }, [queryClient])

  const { data: pending = [], isLoading: loadingPending } = useQuery({
    queryKey: ['actions-pending'],
    queryFn:  api.getActionsPending,
    refetchInterval: 10_000,
  })

  const { data: all = [], isLoading: loadingAll } = useQuery({
    queryKey: ['actions-all'],
    queryFn:  api.getActionsAll,
    enabled:  tab === 'history',
    refetchInterval: false,
  })

  function invalidate() {
    queryClient.invalidateQueries({ queryKey: ['actions-pending'] })
    queryClient.invalidateQueries({ queryKey: ['actions-all'] })
  }

  const history = all.filter(a => a.status !== 'pending')

  return (
    <Page>

      {/* ── En-tête ────────────────────────────────────────────────────────── */}
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <PageTitle>Actions MCP</PageTitle>
        <SseStatus>
          <LiveDot $active={sseConnected} />
          {sseConnected ? 'Temps réel actif' : 'Connexion...'}
        </SseStatus>
      </div>

      {/* ── 1. Autorisations MCP ───────────────────────────────────────────── */}
      <SectionLabel>Autorisations</SectionLabel>
      <PermSection>
        <PermHeader>
          <PermTitle>Validation manuelle par outil</PermTitle>
          <PermDesc>
            Activez le contrôle manuel pour les outils où vous voulez approuver chaque action avant exécution.
            Par défaut, tout s'exécute automatiquement.
          </PermDesc>
        </PermHeader>
        <PermRow>
          <div>
            <PermLabel>Notion</PermLabel>
            <PermHint>Créer / modifier des pages</PermHint>
          </div>
          <Toggle $on={permNotion} onClick={() => togglePerm('notion', permNotion, setPermNotion)} />
        </PermRow>
        <PermRow>
          <div>
            <PermLabel>GitHub</PermLabel>
            <PermHint>Créer des issues, pull requests…</PermHint>
          </div>
          <Toggle $on={permGithub} onClick={() => togglePerm('github', permGithub, setPermGithub)} />
        </PermRow>
        <PermRow>
          <div>
            <PermLabel>Linear</PermLabel>
            <PermHint>Créer / mettre à jour des issues</PermHint>
          </div>
          <Toggle $on={permLinear} onClick={() => togglePerm('linear', permLinear, setPermLinear)} />
        </PermRow>
        <PermRow>
          <div>
            <PermLabel>Jira</PermLabel>
            <PermHint>Créer des tickets, changer les statuts…</PermHint>
          </div>
          <Toggle $on={permJira} onClick={() => togglePerm('jira', permJira, setPermJira)} />
        </PermRow>
      </PermSection>

      {/* ── 2. Pare-feu de confidentialité ─────────────────────────────────── */}
      <SectionLabel>Confidentialité</SectionLabel>
      <PrivacyPanel />

      {/* ── 3. Sources accessibles à Claude ────────────────────────────────── */}
      <SectionLabel>Sources accessibles à Claude</SectionLabel>
      <PermSection>
        <PermHeader>
          <PermTitle>Contrôle d'accès par source</PermTitle>
          <PermDesc>
            Les sources désactivées sont invisibles pour Claude — ni dans search_memory ni dans les tools dédiés. Par défaut tout est accessible.
          </PermDesc>
        </PermHeader>
        {([
          { key: 'email',    label: 'Gmail',     hint: 'Emails IMAP indexés' },
          { key: 'imessage', label: 'iMessage',  hint: 'SMS et iMessages' },
          { key: 'chrome',   label: 'Chrome',    hint: 'Historique de navigation' },
          { key: 'safari',   label: 'Safari',    hint: 'Historique de navigation' },
          { key: 'notes',    label: 'Notes',     hint: 'Apple Notes' },
          { key: 'calendar', label: 'Calendrier',hint: 'Apple Calendar' },
          { key: 'terminal', label: 'Terminal',  hint: 'Historique zsh' },
          { key: 'file',     label: 'Fichiers',  hint: 'Desktop & Documents' },
          { key: 'notion',   label: 'Notion',    hint: 'Pages indexées' },
          { key: 'github',   label: 'GitHub',    hint: 'Issues & PRs indexées' },
          { key: 'linear',   label: 'Linear',    hint: 'Issues indexées' },
          { key: 'jira',     label: 'Jira',      hint: 'Tickets indexés' },
        ] as { key: SourceKey; label: string; hint: string }[]).map(({ key, label, hint }) => (
          <PermRow key={key}>
            <div>
              <PermLabel>{label}</PermLabel>
              <PermHint>{hint}</PermHint>
            </div>
            <Toggle $on={sources[key]} onClick={() => toggleSource(key)} />
          </PermRow>
        ))}
      </PermSection>

      {/* ── 5. Liste noire ─────────────────────────────────────────────────── */}
      <SectionLabel>Liste noire</SectionLabel>
      <BlacklistCard>
        <BlacklistLeft>
          <BlacklistTitle>Éléments bannis</BlacklistTitle>
          <BlacklistDesc>
            Documents, expéditeurs ou domaines exclus des résultats envoyés à Claude.
          </BlacklistDesc>
        </BlacklistLeft>
        <div style={{ display: 'flex', alignItems: 'center' }}>
          <BlacklistCount $n={bannedCount}>
            {bannedCount} banni{bannedCount !== 1 ? 's' : ''}
          </BlacklistCount>
          <ManageBtn onClick={() => setShowBlacklist(true)}>Gérer</ManageBtn>
        </div>
      </BlacklistCard>

      {showBlacklist && (
        <BlacklistPanel source="all" onClose={() => setShowBlacklist(false)} />
      )}

      {/* ── 6. Flux d'actions ──────────────────────────────────────────────── */}
      <SectionLabel>Flux d'actions</SectionLabel>
      <ActionsBlock>
        <ActionsHeader>
          <TabRow>
            <Tab $active={tab === 'pending'} onClick={() => setTab('pending')}>
              En attente
              {pending.length > 0 && <BadgeCount>{pending.length}</BadgeCount>}
            </Tab>
            <Tab $active={tab === 'history'} onClick={() => setTab('history')}>
              Historique
            </Tab>
          </TabRow>
        </ActionsHeader>

        {tab === 'pending' && (
          <>
            {loadingPending && <Loader />}
            {!loadingPending && pending.length === 0 && (
              <EmptyMsg>
                Aucune action en attente.<br />
                Claude soumettra ici ses demandes d'actions pour validation.
              </EmptyMsg>
            )}
            <CardList>
              {pending.map(action => (
                <ActionCardItem key={action.id} action={action} onDecision={invalidate} />
              ))}
            </CardList>
          </>
        )}

        {tab === 'history' && (
          <>
            {loadingAll && <Loader />}
            {!loadingAll && history.length === 0 && (
              <EmptyMsg>Aucune action dans l'historique.</EmptyMsg>
            )}
            <CardList>
              {history.map(action => (
                <ActionCardItem key={action.id} action={action} onDecision={invalidate} />
              ))}
            </CardList>
          </>
        )}
      </ActionsBlock>

    </Page>
  )
}
