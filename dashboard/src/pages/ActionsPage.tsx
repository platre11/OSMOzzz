import { useState, useEffect, useRef } from 'react'
import styled, { keyframes } from 'styled-components'
import { useQuery, useQueryClient, useMutation } from '@tanstack/react-query'
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

// ── Tableau sources unifié ───────────────────────────────────────────────────

const SourceTable = styled.table`width: 100%; border-collapse: collapse;`

const SourceTh = styled.th<{ $center?: boolean }>`
  text-align: ${({ $center }) => $center ? 'center' : 'left'};
  font-size: 11px; font-weight: 600; color: #9ca3af;
  text-transform: uppercase; letter-spacing: .05em;
  padding: 0 20px 12px; border-bottom: 1px solid #f3f4f6;
`

const SourceTd = styled.td<{ $center?: boolean }>`
  padding: 12px 20px; border-bottom: 1px solid #f9fafb;
  text-align: ${({ $center }) => $center ? 'center' : 'left'};
  vertical-align: middle;
  &:last-child { border-bottom: none; }
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

// ── Alias Engine ─────────────────────────────────────────────────────────────

const AliasTable = styled.table`width: 100%; border-collapse: collapse;`
const AliasTh = styled.th`
  text-align: left; font-size: 11px; font-weight: 600; color: #9ca3af;
  text-transform: uppercase; letter-spacing: .05em;
  padding: 0 12px 10px; border-bottom: 1px solid #f3f4f6;
`
const AliasTd = styled.td`
  padding: 9px 12px; font-size: 13px; color: #1a1d23;
  border-bottom: 1px solid #f9fafb; vertical-align: middle;
`
const AliasArrow = styled(AliasTd)`color: #d1d5db; font-size: 16px; width: 32px; text-align: center;`
const AliasMuted = styled(AliasTd)`color: #6b7280;`
const AliasDelBtn = styled.button`
  background: none; border: 1px solid #fca5a5; color: #ef4444;
  border-radius: 6px; padding: 3px 10px; font-size: 12px; cursor: pointer;
  &:hover { background: #fef2f2; }
`
const AliasAddRow = styled.div`display: flex; gap: 8px; margin-top: 14px; align-items: center;`
const AliasInput = styled.input`
  flex: 1; border: 1px solid #e8eaed; border-radius: 8px;
  padding: 7px 11px; font-size: 13px; color: #1a1d23; outline: none;
  &:focus { border-color: #5b5ef4; box-shadow: 0 0 0 3px rgba(91,94,244,.08); }
  &::placeholder { color: #9ca3af; }
`
const AliasAddBtn = styled.button`
  background: #5b5ef4; color: #fff; border: none; border-radius: 8px;
  padding: 7px 16px; font-size: 13px; font-weight: 600; cursor: pointer; white-space: nowrap;
  &:hover { opacity: .88; } &:disabled { opacity: .4; cursor: default; }
`
const AliasSaveBtn = styled.button`
  background: #5b5ef4; color: #fff; border: none; border-radius: 8px;
  padding: 8px 20px; font-size: 13px; font-weight: 600; cursor: pointer;
  &:hover { opacity: .88; } &:disabled { opacity: .4; cursor: default; }
`

// ── Journal ──────────────────────────────────────────────────────────────────

const JournalList = styled.div`display: flex; flex-direction: column; gap: 6px;`

const JournalRow = styled.div<{ $blocked: boolean }>`
  display: flex; align-items: center; gap: 12px;
  padding: 10px 16px; border-radius: 10px; background: #fff;
  border: 1px solid ${({ $blocked }) => $blocked ? '#fee2e2' : '#f3f4f6'};
`

const JournalTime = styled.span`font-size: 11px; color: #9ca3af; white-space: nowrap; min-width: 80px;`

const JournalTool = styled.span`
  font-size: 10px; font-weight: 700; text-transform: uppercase; letter-spacing: .05em;
  padding: 2px 8px; border-radius: 5px; background: #ededff; color: #5b5ef4; white-space: nowrap;
`

const JournalQuery = styled.span`font-size: 13px; color: #374151; flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;`

const JournalCount = styled.span<{ $blocked: boolean }>`
  font-size: 11px; font-weight: 600; white-space: nowrap;
  color: ${({ $blocked }) => $blocked ? '#dc2626' : '#6b7280'};
`

const JournalDataBtn = styled.button`
  font-size: 10px; font-weight: 600; padding: 2px 8px; border-radius: 5px;
  border: 1px solid #e5e7eb; background: #f9fafb; color: #6b7280;
  cursor: pointer; white-space: nowrap;
  &:hover { background: #f3f4f6; color: #374151; }
`

const JournalData = styled.pre`
  margin: 8px 0 0; padding: 10px 14px; border-radius: 8px;
  background: #f8fafc; border: 1px solid #e8eaed;
  font-size: 11px; color: #374151; line-height: 1.5;
  white-space: pre-wrap; word-break: break-word;
  max-height: 300px; overflow-y: auto; font-family: 'SF Mono', monospace;
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

// ─── Composant entrée journal ─────────────────────────────────────────────────

// Clés contenant du contenu textuel utile (toutes sources)
const TEXT_KEYS = new Set([
  'plain_text', 'content', 'text', 'title', 'name', 'body', 'description',
  'summary', 'snippet', 'message', 'subject', 'label', 'value', 'display_name',
  'url', 'html_url', 'web_url', 'created_time', 'last_edited_time',
  'created_at', 'updated_at', 'closed_at', 'state', 'status', 'number',
])

// Clés à ignorer complètement (métadonnées techniques)
const SKIP_KEYS = new Set([
  'id', 'object', 'type', 'color', 'href', 'annotations', 'bold', 'italic',
  'strikethrough', 'underline', 'code', 'created_by', 'last_edited_by',
  'cover', 'icon', 'in_trash', 'is_archived', 'is_locked', 'public_url',
  'archived', 'link', 'has_children', 'parent', 'user', 'workspace',
  'node_id', 'sha', 'permissions', 'owner', 'private', 'fork', 'forks_count',
  'stargazers_count', 'watchers_count', 'open_issues_count', 'default_branch',
  'annotations_count', 'format',
])

// Détecte si une string ressemble à un UUID / ID technique
function looksLikeId(s: string): boolean {
  return /^[0-9a-f-]{32,}$/i.test(s) || /^[0-9a-f]{8}-[0-9a-f]{4}-/i.test(s)
}

function extractTextFromJson(obj: unknown, depth = 0): string[] {
  if (depth > 10) return []
  const lines: string[] = []

  if (typeof obj === 'string') {
    if (obj.length > 2 && !looksLikeId(obj)) lines.push(obj)
    return lines
  }
  if (Array.isArray(obj)) {
    for (const item of obj) lines.push(...extractTextFromJson(item, depth + 1))
    return lines
  }
  if (obj && typeof obj === 'object') {
    const record = obj as Record<string, unknown>
    for (const [key, val] of Object.entries(record)) {
      if (SKIP_KEYS.has(key)) continue
      if (TEXT_KEYS.has(key)) {
        if (typeof val === 'string' && val.length > 0 && !looksLikeId(val)) {
          // Format dates ISO en lisible
          if ((key === 'created_time' || key === 'created_at') && val.includes('T')) {
            lines.push(`Créé : ${new Date(val).toLocaleString('fr-FR')}`)
          } else if ((key === 'last_edited_time' || key === 'updated_at') && val.includes('T')) {
            lines.push(`Modifié : ${new Date(val).toLocaleString('fr-FR')}`)
          } else if (key === 'url' || key === 'html_url' || key === 'web_url') {
            lines.push(`Lien : ${val}`)
          } else if (key === 'state' || key === 'status') {
            lines.push(`Statut : ${val}`)
          } else if (key === 'number') {
            lines.push(`N° ${val}`)
          } else {
            lines.push(val)
          }
        } else if (typeof val === 'number') {
          if (key === 'number') lines.push(`N° ${val}`)
        } else if (Array.isArray(val) || (val && typeof val === 'object')) {
          lines.push(...extractTextFromJson(val, depth + 1))
        }
      } else {
        lines.push(...extractTextFromJson(val, depth + 1))
      }
    }
  }
  return lines
}

function formatJournalData(raw: string): string {
  try {
    const parsed = JSON.parse(raw)
    const lines = extractTextFromJson(parsed)
    // Déduplique et filtre les lignes vides/trop courtes
    const seen = new Set<string>()
    const unique = lines.filter(l => {
      const t = l.trim()
      if (t.length < 2 || seen.has(t)) return false
      seen.add(t)
      return true
    })
    return unique.length > 0 ? unique.join('\n') : raw
  } catch {
    return raw
  }
}

function JournalEntryRow({ entry }: {
  entry: { ts: number; tool: string; query: string; results: number; blocked: boolean; data?: string }
}) {
  const [expanded, setExpanded] = useState(false)
  const date = new Date(entry.ts * 1000)
  const time = date.toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' })
  const day  = date.toLocaleDateString('fr-FR', { day: '2-digit', month: '2-digit' })
  const toolLabel = entry.tool.replace(/^search_/, '').replace(/_/g, ' ')
  const displayData = entry.data ? formatJournalData(entry.data) : ''

  return (
    <div>
      <JournalRow $blocked={entry.blocked}>
        <JournalTime>{day} {time}</JournalTime>
        <JournalTool>{toolLabel}</JournalTool>
        <JournalQuery title={entry.query}>{entry.query || '—'}</JournalQuery>
        <JournalCount $blocked={entry.blocked}>
          {entry.blocked ? '⛔ bloqué' : `${entry.results} résultat${entry.results !== 1 ? 's' : ''}`}
        </JournalCount>
        {entry.data && !entry.blocked && (
          <JournalDataBtn onClick={() => setExpanded(v => !v)}>
            {expanded ? '▲ Masquer' : '▼ Voir'}
          </JournalDataBtn>
        )}
      </JournalRow>
      {expanded && entry.data && (
        <JournalData>{displayData}</JournalData>
      )}
    </div>
  )
}

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
  const [tab, setTab] = useState<'pending' | 'history' | 'journal'>('pending')
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
  const [permEmail,  setPermEmail]  = useState(false)

  useEffect(() => {
    if (permsData) {
      setPermNotion(permsData.notion ?? false)
      setPermGithub(permsData.github ?? false)
      setPermLinear(permsData.linear ?? false)
      setPermJira(permsData.jira ?? false)
      setPermEmail(permsData.email ?? false)
    }
  }, [permsData])

  function togglePerm(
    key: 'notion' | 'github' | 'linear' | 'jira' | 'email',
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
      email:  key === 'email'  ? next : permEmail,
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

  // ── Alias Engine ────────────────────────────────────────────────────────
  const { data: serverAliases } = useQuery({ queryKey: ['aliases'], queryFn: api.getAliases })
  const [aliases, setAliases] = useState<Array<{ real: string; alias: string }>>([])
  const [aliasesDirty, setAliasesDirty] = useState(false)
  const [newReal, setNewReal] = useState('')
  const [newAlias, setNewAlias] = useState('')
  useEffect(() => {
    if (serverAliases !== undefined && !aliasesDirty) {
      setAliases(serverAliases)
    }
  }, [serverAliases])
  const { mutate: saveAliases, isPending: savingAliases } = useMutation({
    mutationFn: () => api.saveAliases(aliases),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['aliases'] })
      setAliasesDirty(false)
    },
  })
  function addAlias() {
    const real = newReal.trim(); const alias = newAlias.trim()
    if (!real || !alias || aliases.some(a => a.real === real)) return
    setAliases(prev => [...prev, { real, alias }])
    setAliasesDirty(true)
    setNewReal(''); setNewAlias('')
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

  const { data: auditEntries = [], isLoading: loadingAudit } = useQuery({
    queryKey: ['audit'],
    queryFn:  () => api.getAudit(200),
    enabled:  tab === 'journal',
    refetchInterval: tab === 'journal' ? 5_000 : false,
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

      {/* ── 1. Tableau sources unifié ──────────────────────────────────────── */}
      <SectionLabel>Contrôle des sources</SectionLabel>
      <PermSection>
        <PermHeader>
          <PermTitle>Accès et validation par source</PermTitle>
          <PermDesc>
            Contrôlez quelles sources sont accessibles à Claude et lesquelles nécessitent une validation manuelle avant exécution.
          </PermDesc>
        </PermHeader>
        <div style={{ padding: '8px 0' }}>
          <SourceTable>
            <thead>
              <tr>
                <SourceTh>Source</SourceTh>
                <SourceTh $center>Accès Claude</SourceTh>
                <SourceTh $center>Validation manuelle</SourceTh>
              </tr>
            </thead>
            <tbody>
              {([
                { key: 'email',    label: 'Gmail',      hint: 'Emails IMAP indexés',       perm: 'email'  },
                { key: 'imessage', label: 'iMessage',   hint: 'SMS et iMessages',           perm: null     },
                { key: 'chrome',   label: 'Chrome',     hint: 'Historique de navigation',   perm: null     },
                { key: 'safari',   label: 'Safari',     hint: 'Historique de navigation',   perm: null     },
                { key: 'notes',    label: 'Notes',      hint: 'Apple Notes',                perm: null     },
                { key: 'calendar', label: 'Calendrier', hint: 'Apple Calendar',             perm: null     },
                { key: 'terminal', label: 'Terminal',   hint: 'Historique zsh',             perm: null     },
                { key: 'file',     label: 'Fichiers',   hint: 'Desktop & Documents',        perm: null     },
                { key: 'notion',   label: 'Notion',     hint: 'Pages indexées',             perm: 'notion' },
                { key: 'github',   label: 'GitHub',     hint: 'Issues & PRs indexées',      perm: 'github' },
                { key: 'linear',   label: 'Linear',     hint: 'Issues indexées',            perm: 'linear' },
                { key: 'jira',     label: 'Jira',       hint: 'Tickets indexés',            perm: 'jira'   },
              ] as { key: SourceKey; label: string; hint: string; perm: 'email'|'notion'|'github'|'linear'|'jira'|null }[]).map(({ key, label, hint, perm }) => (
                <tr key={key}>
                  <SourceTd>
                    <PermLabel>{label}</PermLabel>
                    <PermHint>{hint}</PermHint>
                  </SourceTd>
                  <SourceTd $center>
                    <Toggle $on={sources[key]} onClick={() => toggleSource(key)} />
                  </SourceTd>
                  <SourceTd $center>
                    {perm === 'email'  && <Toggle $on={permEmail}  onClick={() => togglePerm('email',  permEmail,  setPermEmail)}  />}
                    {perm === 'notion' && <Toggle $on={permNotion} onClick={() => togglePerm('notion', permNotion, setPermNotion)} />}
                    {perm === 'github' && <Toggle $on={permGithub} onClick={() => togglePerm('github', permGithub, setPermGithub)} />}
                    {perm === 'linear' && <Toggle $on={permLinear} onClick={() => togglePerm('linear', permLinear, setPermLinear)} />}
                    {perm === 'jira'   && <Toggle $on={permJira}   onClick={() => togglePerm('jira',   permJira,   setPermJira)}   />}
                  </SourceTd>
                </tr>
              ))}
            </tbody>
          </SourceTable>
        </div>
      </PermSection>

      {/* ── 2. Pare-feu de confidentialité ─────────────────────────────────── */}
      <SectionLabel>Confidentialité</SectionLabel>
      <PrivacyPanel />

      {/* ── 3. Alias d'identité ─────────────────────────────────────────────── */}
      <SectionLabel>Alias d'identité</SectionLabel>
      <PermSection>
        <PermHeader>
          <PermTitle>Pseudonymisation des données</PermTitle>
          <PermDesc>
            Remplace les vrais noms par des alias avant envoi à Claude. Claude travaille avec l'alias — il ne voit jamais l'identité réelle. Si Claude cherche un alias, OSMOzzz retrouve le vrai nom dans le vault.
          </PermDesc>
        </PermHeader>
        <div style={{ padding: '16px 20px' }}>
          <AliasTable>
            <thead>
              <tr>
                <AliasTh>Vrai nom / identifiant</AliasTh>
                <AliasTh style={{ width: 32 }} />
                <AliasTh>Alias vu par Claude</AliasTh>
                <AliasTh style={{ width: 90 }} />
              </tr>
            </thead>
            <tbody>
              {aliases.length === 0 && (
                <tr><td colSpan={4} style={{ textAlign: 'center', padding: '20px', color: '#9ca3af', fontSize: 13 }}>
                  Aucun alias défini.
                </td></tr>
              )}
              {aliases.map(({ real, alias }) => (
                <tr key={real}>
                  <AliasTd><strong>{real}</strong></AliasTd>
                  <AliasArrow>→</AliasArrow>
                  <AliasMuted>{alias}</AliasMuted>
                  <AliasTd style={{ textAlign: 'right' }}>
                    <AliasDelBtn onClick={() => { setAliases(prev => prev.filter(a => a.real !== real)); setAliasesDirty(true) }}>
                      Supprimer
                    </AliasDelBtn>
                  </AliasTd>
                </tr>
              ))}
            </tbody>
          </AliasTable>
          <AliasAddRow>
            <AliasInput placeholder="Vrai nom (ex: Jean Pierre)" value={newReal}
              onChange={e => setNewReal(e.target.value)}
              onKeyDown={e => e.key === 'Enter' && addAlias()} />
            <span style={{ color: '#d1d5db', fontSize: 18 }}>→</span>
            <AliasInput placeholder="Alias (ex: Matisse Mouseu)" value={newAlias}
              onChange={e => setNewAlias(e.target.value)}
              onKeyDown={e => e.key === 'Enter' && addAlias()} />
            <AliasAddBtn onClick={addAlias} disabled={!newReal.trim() || !newAlias.trim()}>Ajouter</AliasAddBtn>
          </AliasAddRow>
          <div style={{ display: 'flex', justifyContent: 'flex-end', marginTop: 14 }}>
            <AliasSaveBtn onClick={() => saveAliases()} disabled={savingAliases || !aliasesDirty}>
              {savingAliases ? 'Enregistrement…' : 'Enregistrer'}
            </AliasSaveBtn>
          </div>
        </div>
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
            <Tab $active={tab === 'journal'} onClick={() => setTab('journal')}>
              Journal d'accès
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

        {tab === 'journal' && (
          <>
            {loadingAudit && <Loader />}
            {!loadingAudit && auditEntries.length === 0 && (
              <EmptyMsg>Aucune activité enregistrée.<br />Le journal se remplit dès que Claude utilise un tool OSMOzzz.</EmptyMsg>
            )}
            <JournalList>
              {auditEntries.map((entry, i) => (
                <JournalEntryRow key={i} entry={entry} />
              ))}
            </JournalList>
          </>
        )}
      </ActionsBlock>

    </Page>
  )
}
