import { useState, useEffect, useRef } from 'react'
import { Zap, PlugZap, Shield, Database, RefreshCw } from 'lucide-react'
import styled, { keyframes } from 'styled-components'
import { useQuery, useQueryClient, useMutation } from '@tanstack/react-query'
import { api } from '../api'
import type { ActionRequest, ActionEvent, DbSecurityConfig, ColumnRule } from '../api'
import { PrivacyPanel } from '../components/PrivacyPanel'

// ─── Styles ───────────────────────────────────────────────────────────────────

const spin = keyframes`to { transform: rotate(360deg); }`

const Layout = styled.div`display: flex; flex-direction: column; gap: 0; min-height: 0;`

const TopTabBar = styled.div`
  display: flex; align-items: center; gap: 2px;
  border-bottom: 1px solid #e8eaed;
  margin-bottom: 24px;
  padding-bottom: 0;
`

const TopTabItem = styled.button<{ $active: boolean }>`
  display: flex; align-items: center; gap: 7px;
  padding: 8px 14px 10px; border: none; cursor: pointer; background: transparent;
  font-size: 13px; font-weight: ${({ $active }) => $active ? '600' : '500'};
  color: ${({ $active }) => $active ? '#1a1d23' : '#6b7280'};
  border-bottom: 2px solid ${({ $active }) => $active ? '#5b5ef4' : 'transparent'};
  margin-bottom: -1px;
  transition: color .15s, border-color .15s;
  &:hover { color: #1a1d23; }
`

const SseStatusInline = styled.span`
  margin-left: auto; display: flex; align-items: center; gap: 6px;
  font-size: 11px; color: #9ca3af; padding-bottom: 10px;
`

const Content = styled.div`min-width: 0;`

const SideNavItem = styled.button<{ $active: boolean }>`
  display: flex; align-items: center; gap: 8px;
  width: 100%; padding: 7px 10px; border-radius: 7px;
  border: none; cursor: pointer; text-align: left;
  font-size: 12px; font-weight: ${({ $active }) => $active ? '600' : '500'};
  background: ${({ $active }) => $active ? '#ededff' : 'transparent'};
  color: ${({ $active }) => $active ? '#5b5ef4' : '#6b7280'};
  transition: all .15s;
  &:hover { background: ${({ $active }) => $active ? '#ededff' : '#f3f4f6'}; }
`

const ContentHeader = styled.div`
  display: flex; align-items: center; justify-content: space-between;
  margin-bottom: 20px;
`

const PageTitle = styled.h1`font-size: 18px; font-weight: 700; color: #1a1d23; letter-spacing: -.02em;`

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


const JournalData = styled.pre`
  margin: 8px 0 0; padding: 10px 14px; border-radius: 8px;
  background: #f8fafc; border: 1px solid #e8eaed;
  font-size: 11px; color: #374151; line-height: 1.5;
  white-space: pre-wrap; word-break: break-word;
  max-height: 300px; overflow-y: auto; font-family: 'SF Mono', monospace;
`

// ── Actions (tabs + cards) ───────────────────────────────────────────────────

const ActionsBlock = styled.div`display: flex; flex-direction: column; gap: 12px;`


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


const ExecResult = styled.div<{ $ok: boolean }>`
  margin-top: 12px; padding: 10px 14px; border-radius: 8px; font-size: 12px; font-weight: 500;
  background: ${({ $ok }) => $ok ? '#d1fae5' : '#fee2e2'};
  color: ${({ $ok }) => $ok ? '#065f46' : '#991b1b'};
`

// ── DB Security ──────────────────────────────────────────────────────────────

const DbTableCard = styled.div`
  background: #fff; border: 1px solid #e8eaed; border-radius: 14px;
  overflow: hidden; box-shadow: 0 1px 3px rgba(0,0,0,.05); margin-bottom: 12px;
`

const DbTableHeader = styled.div`
  display: flex; align-items: center; justify-content: space-between;
  padding: 14px 18px; border-bottom: 1px solid #f3f4f6;
  background: #fafafa;
`

const DbTableName = styled.span`font-size: 13px; font-weight: 700; color: #1a1d23; font-family: 'SF Mono', monospace;`

const DbColRow = styled.div`
  display: flex; align-items: center; padding: 10px 18px;
  border-bottom: 1px solid #f9fafb;
  &:last-child { border-bottom: none; }
`

const DbColName = styled.span`font-size: 13px; font-weight: 500; color: #374151; flex: 1; font-family: 'SF Mono', monospace; font-size: 12px;`

const DbColType = styled.span`font-size: 11px; color: #9ca3af; margin-right: 16px; min-width: 80px;`

const RuleSelector = styled.div`display: flex; gap: 4px;`

const RuleBtn = styled.button<{ $active: boolean; $variant: 'free' | 'tokenize' | 'block' }>`
  padding: 4px 10px; border-radius: 6px; font-size: 11px; font-weight: 600;
  border: 1px solid ${({ $active, $variant }) =>
    !$active ? '#e5e7eb' :
    $variant === 'free'     ? '#10b981' :
    $variant === 'tokenize' ? '#f59e0b' :
    '#ef4444'};
  background: ${({ $active, $variant }) =>
    !$active ? '#fff' :
    $variant === 'free'     ? '#d1fae5' :
    $variant === 'tokenize' ? '#fef3c7' :
    '#fee2e2'};
  color: ${({ $active, $variant }) =>
    !$active ? '#9ca3af' :
    $variant === 'free'     ? '#065f46' :
    $variant === 'tokenize' ? '#92400e' :
    '#991b1b'};
  cursor: pointer; transition: all .12s;
  &:hover { opacity: .8; }
`

const DbToolbar = styled.div`display: flex; align-items: center; gap: 10px; margin-bottom: 20px;`

const DbProjectBadge = styled.div`
  display: flex; align-items: center; gap: 6px;
  padding: 6px 12px; border-radius: 8px; font-size: 12px; font-weight: 600;
  background: #ededff; color: #5b5ef4; border: 1px solid #d8d8ff;
`

const DbDeleteBtn = styled.button`
  padding: 7px 14px; border-radius: 8px; font-size: 13px; font-weight: 500;
  border: 1px solid #fecaca; background: #fff5f5; color: #ef4444; cursor: pointer;
  transition: all .15s;
  &:hover { background: #fee2e2; }
`

const ProjectSelect = styled.select`
  padding: 7px 12px; border: 1px solid #e8eaed; border-radius: 9px;
  font-size: 13px; color: #1a1d23; background: #fff; outline: none; cursor: pointer;
  &:focus { border-color: #5b5ef4; box-shadow: 0 0 0 3px rgba(91,94,244,.08); }
  &:disabled { opacity: .5; cursor: not-allowed; }
`

const DbEmptyMsg = styled.p`
  text-align: center; padding: 48px 24px; color: #9ca3af; font-size: 13px; line-height: 1.6;
`

const DbErrorMsg = styled.p`
  padding: 12px 16px; border-radius: 9px; background: #fee2e2; color: #991b1b; font-size: 13px; margin-bottom: 12px;
`

const DbSavedMsg = styled.span`font-size: 12px; color: #10b981; font-weight: 600;`

const LegendRow = styled.div`display: flex; gap: 16px; margin-bottom: 16px; flex-wrap: wrap;`

const LegendItem = styled.div`display: flex; align-items: center; gap: 6px; font-size: 11px; color: #6b7280;`

const LegendDot = styled.span<{ $color: string }>`
  width: 8px; height: 8px; border-radius: 50%; background: ${({ $color }) => $color};
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

  const clickable = !!entry.data && !entry.blocked

  return (
    <div>
      <JournalRow
        $blocked={entry.blocked}
        onClick={clickable ? () => setExpanded(v => !v) : undefined}
        style={clickable ? { cursor: 'pointer' } : undefined}
      >
        <JournalTime>{day} {time}</JournalTime>
        <JournalTool>{toolLabel}</JournalTool>
        <JournalQuery title={entry.query}>{entry.query || '—'}</JournalQuery>
        {entry.blocked && <JournalCount $blocked>{' ⛔'}</JournalCount>}
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
  type Section = 'flux' | 'sources' | 'privacy' | 'database'
  const [activeSection, setActiveSection] = useState<Section>('flux')
  const [sseConnected, setSseConnected] = useState(false)
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
  type AliasEntry = { real: string; alias: string; alias_type?: string }
  const { data: serverAliasData } = useQuery({ queryKey: ['aliases'], queryFn: api.getAliases })
  const [aliases, setAliases] = useState<AliasEntry[]>([])
  const [aliasTypes, setAliasTypes] = useState<string[]>([])
  const [aliasesDirty, setAliasesDirty] = useState(false)
  const [selectedType, setSelectedType] = useState<string | null>(null)
  const [newReal, setNewReal] = useState('')
  const [newAlias, setNewAlias] = useState('')
  const [newTypeName, setNewTypeName] = useState('')
  const [showAddType, setShowAddType] = useState(false)
  useEffect(() => {
    if (serverAliasData !== undefined && !aliasesDirty) {
      setAliases(serverAliasData.aliases)
      setAliasTypes(serverAliasData.types)
      if (serverAliasData.types.length > 0) setSelectedType(t => t ?? serverAliasData.types[0])
    }
  }, [serverAliasData])
  const { mutate: saveAliases, isPending: savingAliases } = useMutation({
    mutationFn: (payload: { aliases: AliasEntry[]; types: string[] }) =>
      api.saveAliases(payload.aliases, payload.types),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['aliases'] })
      setAliasesDirty(false)
    },
  })

  function persistNow(nextAliases: AliasEntry[], nextTypes: string[]) {
    saveAliases({ aliases: nextAliases, types: nextTypes })
  }

  function addAlias() {
    const real = newReal.trim(); const alias = newAlias.trim()
    // Doublon uniquement au sein du même type (pas global)
    if (!real || !alias || aliases.some(a => a.real === real && a.alias_type === (selectedType ?? undefined))) return
    const next = [...aliases, { real, alias, alias_type: selectedType ?? undefined }]
    setAliases(next)
    setAliasesDirty(true)
    setNewReal(''); setNewAlias('')
    persistNow(next, aliasTypes)
  }
  function removeAlias(real: string, aliasType?: string) {
    const next = aliases.filter(a => !(a.real === real && a.alias_type === aliasType))
    setAliases(next)
    persistNow(next, aliasTypes)
  }
  function addType() {
    const t = newTypeName.trim()
    if (!t || aliasTypes.includes(t)) return
    const nextTypes = [...aliasTypes, t]
    setAliasTypes(nextTypes)
    setNewTypeName(''); setShowAddType(false)
    if (selectedType === null) setSelectedType(t)
    persistNow(aliases, nextTypes)
  }
  function deleteType(t: string) {
    const nextTypes = aliasTypes.filter(x => x !== t)
    const nextAliases = aliases.map(a => a.alias_type === t ? { ...a, alias_type: undefined } : a)
    setAliasTypes(nextTypes)
    setAliases(nextAliases)
    if (selectedType === t) setSelectedType(nextTypes[0] ?? null)
    persistNow(nextAliases, nextTypes)
  }

  // ── DB Security ──────────────────────────────────────────────────────────
  type SupabaseProject = { id: string; name: string; region: string }
  const [dbSecurity, setDbSecurity]   = useState<DbSecurityConfig>({ supabase: {} })
  const [dbSchemaLoading, setDbSchemaLoading] = useState(false)
  const [dbSchemaError, setDbSchemaError]     = useState<string | null>(null)
  const [dbSchemaTables, setDbSchemaTables]   = useState<Array<{ table_name: string; columns: Array<{ column_name: string; data_type: string }> }>>([])
  const [dbSaved, setDbSaved]                 = useState(false)
  const [dbProjects, setDbProjects]           = useState<SupabaseProject[]>([])
  const [dbProjectsLoading, setDbProjectsLoading] = useState(false)
  const [dbActiveProject, setDbActiveProject] = useState<string>('')

  const { data: dbSecurityData } = useQuery({
    queryKey: ['db-security'],
    queryFn:  api.getDbSecurity,
    enabled:  activeSection === 'database',
  })

  // Charge les projets + config sauvegardée à l'ouverture du tab
  useEffect(() => {
    if (activeSection !== 'database') return
    setDbProjectsLoading(true)
    Promise.all([api.getSupabaseProjects(), api.getDbSecurity()])
      .then(([projects, security]) => {
        setDbProjects(projects)
        const saved = security.active_project_id ?? ''
        const match = projects.find(p => p.id === saved)
        setDbActiveProject(match ? match.id : (projects[0]?.id ?? ''))
      })
      .catch(() => {})
      .finally(() => setDbProjectsLoading(false))
  }, [activeSection])

  useEffect(() => {
    if (dbSecurityData) {
      setDbSecurity(dbSecurityData)
      if (dbSchemaTables.length === 0 && Object.keys(dbSecurityData.supabase).length > 0) {
        const order = dbSecurityData.column_order ?? {}
        const synthetic = Object.keys(dbSecurityData.supabase)
          .sort((a, b) => a.localeCompare(b))
          .map(table_name => {
            const cols = dbSecurityData.supabase[table_name]
            const colNames = order[table_name]
              ? order[table_name].filter(c => c in cols)
              : Object.keys(cols).sort()
            return {
              table_name,
              columns: colNames.map((column_name, i) => ({ column_name, data_type: '', ordinal_position: i + 1 })),
            }
          })
        setDbSchemaTables(synthetic)
      }
    }
  }, [dbSecurityData])

  async function importSchema(projectId?: string) {
    setDbSchemaLoading(true); setDbSchemaError(null)
    try {
      if (projectId) await api.saveSupabaseProject(projectId)
      // Tables sorted by name; columns arrive in ordinal_position order from Supabase
      const tables = (await api.getSupabaseSchema()).sort((a, b) => a.table_name.localeCompare(b.table_name))
      setDbSchemaTables(tables)
      setDbSecurity(prev => {
        // Nouveau projet → tables reparties de zéro (pas de merge avec l'ancien projet)
        const next: DbSecurityConfig = {
          active_project_id: projectId ?? prev.active_project_id,
          supabase: {},
          column_order: {},
        }
        for (const t of tables) {
          next.supabase[t.table_name] = {}
          next.column_order![t.table_name] = t.columns.map(c => c.column_name)
          for (const c of t.columns) {
            next.supabase[t.table_name][c.column_name] = 'free'
          }
        }
        api.saveDbSecurity(next).catch(() => {})
        return next
      })
    } catch (e: unknown) {
      const projectName = dbProjects.find(p => p.id === (projectId ?? dbActiveProject))?.name ?? projectId ?? ''
      const base = e instanceof Error ? e.message : 'Erreur inconnue'
      setDbSchemaError(projectName ? `${projectName} — ${base}` : base)
      throw e
    } finally {
      setDbSchemaLoading(false)
    }
  }

  async function onProjectChange(projectId: string) {
    const previous = dbActiveProject
    setDbActiveProject(projectId)
    // Sauvegarder le projet sélectionné immédiatement (même si l'import échoue)
    api.saveDbSecurity({ ...dbSecurity, active_project_id: projectId }).catch(() => {})
    try {
      await importSchema(projectId)
    } catch {
      setDbActiveProject(previous)
      api.saveDbSecurity({ ...dbSecurity, active_project_id: previous }).catch(() => {})
    }
  }

  async function deleteSchema() {
    setDbSchemaTables([])
    const empty: DbSecurityConfig = { supabase: {} }
    setDbSecurity(empty)
    await api.saveDbSecurity(empty)
  }

  async function setColumnRule(table: string, column: string, rule: ColumnRule) {
    const next: DbSecurityConfig = {
      active_project_id: dbSecurity.active_project_id,
      column_order: dbSecurity.column_order,
      supabase: {
        ...dbSecurity.supabase,
        [table]: { ...dbSecurity.supabase[table], [column]: rule },
      },
    }
    setDbSecurity(next)
    try {
      await api.saveDbSecurity(next)
      setDbSaved(true)
      setTimeout(() => setDbSaved(false), 1500)
    } catch { /* ignore */ }
  }

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
    refetchInterval: false,
  })

  const { data: auditEntries = [], isLoading: loadingAudit } = useQuery({
    queryKey: ['audit'],
    queryFn:  () => api.getAudit(200),
    refetchInterval: 5_000,
  })

  function invalidate() {
    queryClient.invalidateQueries({ queryKey: ['actions-pending'] })
    queryClient.invalidateQueries({ queryKey: ['actions-all'] })
  }

  const history = all.filter(a => a.status !== 'pending')

  const NAV_ITEMS = [
    { id: 'flux',      label: 'Flux d\'actions',   Icon: Zap      },
    { id: 'sources',   label: 'Sources',            Icon: PlugZap  },
    { id: 'privacy',   label: 'Confidentialité',    Icon: Shield   },
    { id: 'database',  label: 'Bases de données',   Icon: Database },
  ] as const

  return (
    <Layout>

      {/* ── Tab bar horizontale ── */}
      <TopTabBar>
        {NAV_ITEMS.map(({ id, label, Icon }) => (
          <TopTabItem
            key={id}
            $active={activeSection === id}
            onClick={() => setActiveSection(id)}
          >
            <Icon size={14} />
            {label}
            {id === 'flux' && pending.length > 0 && <BadgeCount>{pending.length}</BadgeCount>}
          </TopTabItem>
        ))}
        <SseStatusInline>
          <LiveDot $active={sseConnected} />
          {sseConnected ? 'Temps réel actif' : 'Connexion...'}
        </SseStatusInline>
      </TopTabBar>

      {/* ── Contenu ── */}
      <Content>

        {/* 1. Flux d'actions */}
        {activeSection === 'flux' && (
          <ActionsBlock>
            <ContentHeader>
              <PageTitle>Flux d'actions</PageTitle>
            </ContentHeader>
            <div style={{ display: 'flex', gap: 0, alignItems: 'flex-start', background: '#fff', border: '1px solid #e8eaed', borderRadius: 14, overflow: 'hidden' }}>

              {/* Colonne En attente */}
              <div style={{ flex: 1, minWidth: 0, padding: '16px 20px', borderRight: '1px solid #e8eaed' }}>
                <div style={{ fontSize: 12, fontWeight: 600, color: '#6b7280', textTransform: 'uppercase', letterSpacing: '.06em', marginBottom: 10, display: 'flex', alignItems: 'center', gap: 6 }}>
                  En attente
                  {pending.length > 0 && <BadgeCount>{pending.length}</BadgeCount>}
                </div>
                {loadingPending && <Loader />}
                {!loadingPending && pending.length === 0 && (
                  <EmptyMsg>Aucune action en attente.<br />Claude soumettra ici ses demandes d'actions pour validation.</EmptyMsg>
                )}
                <CardList>{pending.map(a => <ActionCardItem key={a.id} action={a} onDecision={invalidate} />)}</CardList>
              </div>

              {/* Colonne Historique */}
              <div style={{ flex: 1, minWidth: 0, padding: '16px 20px', borderRight: '1px solid #e8eaed' }}>
                <div style={{ fontSize: 12, fontWeight: 600, color: '#6b7280', textTransform: 'uppercase', letterSpacing: '.06em', marginBottom: 10 }}>
                  Historique
                </div>
                {loadingAll && <Loader />}
                {!loadingAll && history.length === 0 && <EmptyMsg>Aucune action dans l'historique.</EmptyMsg>}
                <CardList>{history.map(a => <ActionCardItem key={a.id} action={a} onDecision={invalidate} />)}</CardList>
              </div>

              {/* Colonne Journal d'accès */}
              <div style={{ flex: 1, minWidth: 0, padding: '16px 20px' }}>
                <div style={{ fontSize: 12, fontWeight: 600, color: '#6b7280', textTransform: 'uppercase', letterSpacing: '.06em', marginBottom: 10 }}>
                  Journal d'accès
                </div>
                {loadingAudit && <Loader />}
                {!loadingAudit && auditEntries.length === 0 && (
                  <EmptyMsg>Aucune activité enregistrée.<br />Le journal se remplit dès que Claude utilise un tool OSMOzzz.</EmptyMsg>
                )}
                <JournalList>{auditEntries.map((e, i) => <JournalEntryRow key={i} entry={e} />)}</JournalList>
              </div>

            </div>
          </ActionsBlock>
        )}

        {/* 2. Sources */}
        {activeSection === 'sources' && (
          <>
            <ContentHeader>
              <PageTitle>Contrôle des sources</PageTitle>
            </ContentHeader>
            <PermSection>
              <PermHeader>
                <PermTitle>Accès et validation par source</PermTitle>
                <PermDesc>Contrôlez quelles sources sont accessibles à Claude et lesquelles nécessitent une validation manuelle avant exécution.</PermDesc>
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
                        <SourceTd><PermLabel>{label}</PermLabel><PermHint>{hint}</PermHint></SourceTd>
                        <SourceTd $center><Toggle $on={sources[key]} onClick={() => toggleSource(key)} /></SourceTd>
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
          </>
        )}

        {/* 3. Confidentialité + Alias */}
        {activeSection === 'privacy' && (
          <>
            <ContentHeader>
              <PageTitle>Confidentialité</PageTitle>
            </ContentHeader>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 20 }}>
              <PrivacyPanel />
              <PermSection>
                <PermHeader>
                  <PermTitle>Alias d'identité</PermTitle>
                  <PermDesc>Remplace les vrais noms par des alias avant envoi à Claude. Organisez vos alias par type.</PermDesc>
                </PermHeader>
                <div style={{ display: 'flex', minHeight: 0 }}>
                  {/* Mini sidebar types */}
                  <div style={{ width: 160, borderRight: '1px solid #f3f4f6', padding: '12px 8px', display: 'flex', flexDirection: 'column', gap: 2 }}>
                    {aliasTypes.map(t => (
                      <div key={t} style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
                        <SideNavItem $active={selectedType === t} onClick={() => setSelectedType(t)} style={{ fontSize: 12, padding: '6px 10px', flex: 1 }}>
                          {t} ({aliases.filter(a => a.alias_type === t).length})
                        </SideNavItem>
                        <AliasDelBtn onClick={() => deleteType(t)} style={{ padding: '2px 6px', fontSize: 10 }}>✕</AliasDelBtn>
                      </div>
                    ))}
                    {showAddType ? (
                      <div style={{ padding: '6px 4px', display: 'flex', flexDirection: 'column', gap: 4 }}>
                        <AliasInput placeholder="Nom du type" value={newTypeName}
                          onChange={e => setNewTypeName(e.target.value)}
                          onKeyDown={e => { if (e.key === 'Enter') addType(); if (e.key === 'Escape') setShowAddType(false) }}
                          style={{ fontSize: 12, padding: '5px 8px' }} autoFocus />
                        <AliasAddBtn onClick={addType} disabled={!newTypeName.trim()} style={{ fontSize: 11, padding: '4px 8px' }}>OK</AliasAddBtn>
                      </div>
                    ) : (
                      <button onClick={() => setShowAddType(true)} style={{ background: 'none', border: '1px dashed #d1d5db', borderRadius: 7, padding: '5px 10px', fontSize: 11, color: '#9ca3af', cursor: 'pointer', marginTop: 4, textAlign: 'left' }}>
                        + Nouveau type
                      </button>
                    )}
                  </div>
                  {/* Contenu aliases */}
                  <div style={{ flex: 1, padding: '12px 16px', minWidth: 0 }}>
                    <AliasTable>
                      <thead>
                        <tr>
                          <AliasTh>Vrai nom</AliasTh>
                          <AliasTh style={{ width: 24 }} />
                          <AliasTh>Alias</AliasTh>
                          <AliasTh style={{ width: 80 }} />
                        </tr>
                      </thead>
                      <tbody>
                        {aliases.filter(a => selectedType === null || a.alias_type === selectedType).length === 0 && (
                          <tr><td colSpan={5} style={{ textAlign: 'center', padding: '20px', color: '#9ca3af', fontSize: 13 }}>Aucun alias{selectedType ? ` dans "${selectedType}"` : ''}.</td></tr>
                        )}
                        {aliases
                          .filter(a => selectedType === null || a.alias_type === selectedType)
                          .map(({ real, alias, alias_type }) => (
                            <tr key={`${real}__${alias_type ?? ''}`}>
                              <AliasTd><strong>{real}</strong></AliasTd>
                              <AliasArrow>→</AliasArrow>
                              <AliasMuted>{alias}</AliasMuted>
                              <AliasTd style={{ textAlign: 'right' }}>
                                <AliasDelBtn onClick={() => removeAlias(real, alias_type)}>
                                  Supprimer
                                </AliasDelBtn>
                              </AliasTd>
                            </tr>
                          ))}
                      </tbody>
                    </AliasTable>
                    <AliasAddRow style={{ marginTop: 12 }}>
                      <AliasInput placeholder="Vrai nom" value={newReal}
                        onChange={e => setNewReal(e.target.value)}
                        onKeyDown={e => e.key === 'Enter' && addAlias()} />
                      <span style={{ color: '#d1d5db', fontSize: 16 }}>→</span>
                      <AliasInput placeholder="Alias" value={newAlias}
                        onChange={e => setNewAlias(e.target.value)}
                        onKeyDown={e => e.key === 'Enter' && addAlias()} />
                      <AliasAddBtn onClick={addAlias} disabled={!newReal.trim() || !newAlias.trim() || savingAliases}>
                        {savingAliases ? '…' : 'Ajouter'}
                      </AliasAddBtn>
                    </AliasAddRow>
                  </div>
                </div>
              </PermSection>
            </div>
          </>
        )}

        {/* 5. Bases de données */}
        {activeSection === 'database' && (
          <>
            <ContentHeader>
              <PageTitle>Sécurité des bases de données</PageTitle>
            </ContentHeader>

            <DbToolbar>
              {dbProjectsLoading ? (
                <DbProjectBadge><RefreshCw size={12} style={{ animation: 'spin .7s linear infinite' }} /> Chargement…</DbProjectBadge>
              ) : dbProjects.length > 0 ? (
                <ProjectSelect
                  value={dbActiveProject}
                  onChange={e => onProjectChange(e.target.value)}
                  disabled={dbSchemaLoading}
                  style={{ width: 'auto', padding: '7px 12px', fontSize: 13 }}
                >
                  {dbProjects.map(p => (
                    <option key={p.id} value={p.id}>{p.name}</option>
                  ))}
                </ProjectSelect>
              ) : (
                <DbProjectBadge style={{ color: '#9ca3af', background: '#f9fafb', borderColor: '#e5e7eb' }}>
                  Aucun projet Supabase configuré
                </DbProjectBadge>
              )}

              {dbSchemaLoading && <DbProjectBadge><RefreshCw size={12} style={{ animation: 'spin .7s linear infinite' }} /> Importation…</DbProjectBadge>}

              {dbSchemaTables.length > 0 && !dbSchemaLoading && (
                <DbDeleteBtn onClick={deleteSchema}>Supprimer la structure</DbDeleteBtn>
              )}

              {dbSaved && <DbSavedMsg>✓ Sauvegardé</DbSavedMsg>}
            </DbToolbar>

            {dbSchemaError && <DbErrorMsg>Erreur : {dbSchemaError}</DbErrorMsg>}

            {dbSchemaTables.length > 0 && (
              <LegendRow>
                <LegendItem><LegendDot $color="#10b981" />Libre — valeur brute transmise à Claude</LegendItem>
                <LegendItem><LegendDot $color="#f59e0b" />Tokenisé — remplacé par un token stable (tok_em_…)</LegendItem>
                <LegendItem><LegendDot $color="#ef4444" />Bloqué — colonne visible pour Claude mais valeur masquée ([bloqué])</LegendItem>
              </LegendRow>
            )}

            {dbSchemaTables.length === 0 && !dbSchemaLoading && dbProjects === null && (
              <DbTableCard>
                <DbEmptyMsg>
                  Aucune structure importée.<br />
                  Cliquez sur <strong>Importer la structure Supabase</strong> pour récupérer vos tables et configurer la sécurité colonne par colonne.
                </DbEmptyMsg>
              </DbTableCard>
            )}

            {dbSchemaTables.map(table => (
              <DbTableCard key={table.table_name}>
                <DbTableHeader>
                  <DbTableName>{table.table_name}</DbTableName>
                  <span style={{ fontSize: 11, color: '#9ca3af' }}>{table.columns.length} colonne{table.columns.length !== 1 ? 's' : ''}</span>
                </DbTableHeader>
                {table.columns.map(col => {
                  const rule: ColumnRule = dbSecurity.supabase[table.table_name]?.[col.column_name] ?? 'free'
                  return (
                    <DbColRow key={col.column_name}>
                      <DbColName>{col.column_name}</DbColName>
                      <DbColType>{col.data_type}</DbColType>
                      <RuleSelector>
                        <RuleBtn
                          $active={rule === 'free'} $variant="free"
                          onClick={() => setColumnRule(table.table_name, col.column_name, 'free')}
                        >Libre</RuleBtn>
                        <RuleBtn
                          $active={rule === 'tokenize'} $variant="tokenize"
                          onClick={() => setColumnRule(table.table_name, col.column_name, 'tokenize')}
                        >Tokenisé</RuleBtn>
                        <RuleBtn
                          $active={rule === 'block'} $variant="block"
                          onClick={() => setColumnRule(table.table_name, col.column_name, 'block')}
                        >Bloqué</RuleBtn>
                      </RuleSelector>
                    </DbColRow>
                  )
                })}
              </DbTableCard>
            ))}
          </>
        )}

      </Content>
    </Layout>
  )
}
