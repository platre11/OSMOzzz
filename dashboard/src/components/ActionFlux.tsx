/**
 * Composants partagés entre ActionsPage (actions locales Claude)
 * et NetworkPage (flux P2P de l'ami).
 *
 * ActionCardItem  — carte d'approbation/rejet (EN ATTENTE + HISTORIQUE)
 * JournalEntryRow — ligne du journal d'accès
 */
import { useState, useEffect } from 'react'
import styled, { keyframes } from 'styled-components'
import { Shield, ShieldOff, Key, UserRound } from 'lucide-react'
import type { ActionRequest } from '../api'

// ─── Utilitaires de layout ───────────────────────────────────────────────────

const spin = keyframes`to { transform: rotate(360deg); }`

export const CardList = styled.div`display: flex; flex-direction: column; gap: 12px;`

export const EmptyMsg = styled.p`text-align: center; padding: 60px; color: #9ca3af;`

export const Loader = styled.div`
  width: 20px; height: 20px; border: 2px solid #e5e7eb; border-top-color: #5b5ef4;
  border-radius: 50%; animation: ${spin} .7s linear infinite; margin: 60px auto;
`

export const BadgeCount = styled.span`
  display: inline-flex; align-items: center; justify-content: center;
  min-width: 18px; height: 18px; padding: 0 5px;
  background: #ef4444; color: #fff; border-radius: 99px;
  font-size: 10px; font-weight: 700; margin-left: 6px;
`

// ─── Journal d'accès ─────────────────────────────────────────────────────────

const CONNECTOR_COLORS: Record<string, string> = {
  github: '#24292e', gitlab: '#e24329', linear: '#5e6ad2', jira: '#0052cc',
  sentry: '#362d59', vercel: '#000000', railway: '#c10000', render: '#46e3b7',
  stripe: '#635bff', hubspot: '#ff7a59', discord: '#5865f2', figma: '#f24e1e',
  notion: '#000000', slack: '#4a154b', supabase: '#3ecf8e', cloudflare: '#f38020',
  reddit: '#ff4500', calendly: '#006bff', posthog: '#f54e00', resend: '#000000',
  twilio: '#f22f46', google: '#4285f4', gcal: '#4285f4', gmail: '#ea4335',
  search: '#6b7280', find: '#6b7280', fetch: '#6b7280', get: '#6b7280',
  list: '#6b7280', index: '#6b7280', osmozzz: '#5b5ef4', act: '#f59e0b',
}

export function connectorColor(tool: string): string {
  if (tool.includes(':')) {
    const prefix = tool.split(':')[0].toLowerCase()
    return CONNECTOR_COLORS[prefix] ?? '#5b5ef4'
  }
  const prefix = tool.split('_')[0].toLowerCase()
  return CONNECTOR_COLORS[prefix] ?? '#5b5ef4'
}

export function parseToolDisplay(tool: string): { connector: string; action: string } {
  if (tool.includes(':')) {
    const [connector, ...rest] = tool.split(':')
    return { connector, action: rest.join(':').replace(/_/g, ' ') }
  }
  const parts = tool.split('_')
  const connector = parts[0]
  const action = parts.slice(1).join(' ')
  return { connector, action: action || connector }
}

export const JournalList = styled.div`display: flex; flex-direction: column;`

const JournalRow = styled.div<{ $blocked: boolean; $color: string }>`
  display: flex; flex-direction: column; gap: 3px;
  padding: 8px 10px 8px 14px;
  border-bottom: 1px solid #f3f4f6;
  border-left: 3px solid ${({ $blocked, $color }) => $blocked ? '#dc2626' : $color};
  background: ${({ $blocked }) => $blocked ? '#fff5f5' : 'transparent'};
  transition: background 0.1s;
  &:hover { background: ${({ $blocked }) => $blocked ? '#fff0f0' : '#fafafa'}; }
  &:last-child { border-bottom: none; }
`

const JournalMeta = styled.div`display: flex; align-items: center; gap: 6px; flex-wrap: wrap;`
const JournalTime = styled.span`font-size: 11px; color: #b0b7c3; white-space: nowrap; font-variant-numeric: tabular-nums;`
const JournalConnector = styled.span<{ $color: string }>`font-size: 11px; font-weight: 700; color: ${({ $color }) => $color}; white-space: nowrap; letter-spacing: 0.01em;`
const JournalAction = styled.span`font-size: 11px; font-weight: 400; color: #9ca3af; white-space: nowrap;`
const JournalResultsBadge = styled.span<{ $blocked: boolean }>`
  font-size: 10px; font-weight: 600; padding: 1px 6px; border-radius: 10px;
  background: ${({ $blocked }) => $blocked ? '#fee2e2' : '#f0f0ff'};
  color: ${({ $blocked }) => $blocked ? '#dc2626' : '#6366f1'};
  white-space: nowrap; margin-left: auto;
`
const JournalQuery = styled.span`
  font-size: 12px; color: #4b5563; line-height: 1.5; word-break: break-word;
  padding: 2px 6px; border-radius: 4px; background: #f9fafb;
  display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical;
  overflow: hidden; font-style: italic;
`

const DataBox = styled.div`
  margin: 6px 0 0; padding: 10px 14px; border-radius: 8px;
  background: #f8fafc; border: 1px solid #e8eaed;
  font-size: 11px; color: #374151; line-height: 1.5;
  white-space: pre-wrap; word-break: break-word;
  max-height: 220px; overflow-y: auto; font-family: 'SF Mono', monospace;
`

const SecurityBox = styled.div`margin: 6px 0 0; border-radius: 8px; background: #fffbeb; border: 1px solid #fde68a; overflow: hidden;`
const SecurityBoxHeader = styled.div`display: flex; align-items: center; gap: 6px; padding: 6px 12px; border-bottom: 1px solid #fde68a; font-size: 10px; font-weight: 700; text-transform: uppercase; letter-spacing: .06em; color: #92400e;`
const SecurityGroup = styled.div`padding: 8px 12px 6px; border-bottom: 1px solid #fef3c7; &:last-child { border-bottom: none; }`
const SecurityGroupTitle = styled.div<{ $color: string }>`display: flex; align-items: center; gap: 5px; font-size: 10px; font-weight: 700; text-transform: uppercase; letter-spacing: .05em; color: ${({ $color }) => $color}; margin-bottom: 5px;`
const SecurityMaskList = styled.div`display: flex; flex-direction: column; gap: 2px;`
const SecurityMaskItem = styled.div`display: flex; align-items: baseline; gap: 6px; font-size: 11px; font-family: 'SF Mono', monospace;`
const SecurityMaskNum = styled.span`color: #9ca3af; font-size: 10px; font-weight: 700; flex-shrink: 0; min-width: 22px;`
const SecurityMaskEmail = styled.span`color: #1a1d23;`
const SecurityRow = styled.div`display: flex; align-items: center; gap: 10px; padding: 3px 0;`
const SecurityRealValue = styled.span`font-size: 11px; font-family: 'SF Mono', monospace; color: #1a1d23; flex-shrink: 0;`
const SecurityArrow = styled.span`font-size: 11px; color: #9ca3af; flex-shrink: 0;`
const SecurityReplaced = styled.span<{ $kind: 'block' | 'tokenize' | 'alias' | 'mask' }>`
  font-size: 11px; font-family: 'SF Mono', monospace; font-weight: 600;
  color: ${({ $kind }) => $kind === 'block' || $kind === 'mask' ? '#dc2626' : $kind === 'alias' ? '#059669' : '#d97706'};
`
const SecurityBadge = styled.span<{ $kind: 'block' | 'tokenize' | 'alias' | 'mask' }>`
  font-size: 10px; font-weight: 700; padding: 1px 6px; border-radius: 4px; margin-left: 4px;
  background: ${({ $kind }) => $kind === 'block' || $kind === 'mask' ? '#fee2e2' : $kind === 'alias' ? '#d1fae5' : '#fef3c7'};
  color: ${({ $kind }) => $kind === 'block' || $kind === 'mask' ? '#dc2626' : $kind === 'alias' ? '#059669' : '#d97706'};
`

// Clés contenant du contenu textuel utile
const TEXT_KEYS = new Set(['plain_text','content','text','title','name','body','description','summary','snippet','message','subject','label','value','display_name','url','html_url','web_url','created_time','last_edited_time','created_at','updated_at','closed_at','state','status','number'])
const SKIP_KEYS = new Set(['id','object','type','color','href','annotations','bold','italic','strikethrough','underline','code','created_by','last_edited_by','cover','icon','in_trash','is_archived','is_locked','public_url','archived','link','has_children','parent','user','workspace','node_id','sha','permissions','owner','private','fork','forks_count','stargazers_count','watchers_count','open_issues_count','default_branch','annotations_count','format'])

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
  if (Array.isArray(obj)) { for (const item of obj) lines.push(...extractTextFromJson(item, depth + 1)); return lines }
  if (obj && typeof obj === 'object') {
    const record = obj as Record<string, unknown>
    for (const [key, val] of Object.entries(record)) {
      if (SKIP_KEYS.has(key)) continue
      if (TEXT_KEYS.has(key)) {
        if (typeof val === 'string' && val.length > 0 && !looksLikeId(val)) {
          if ((key === 'created_time' || key === 'created_at') && val.includes('T')) lines.push(`Créé : ${new Date(val).toLocaleString('fr-FR')}`)
          else if ((key === 'last_edited_time' || key === 'updated_at') && val.includes('T')) lines.push(`Modifié : ${new Date(val).toLocaleString('fr-FR')}`)
          else if (key === 'url' || key === 'html_url' || key === 'web_url') lines.push(`Lien : ${val}`)
          else if (key === 'state' || key === 'status') lines.push(`Statut : ${val}`)
          else if (key === 'number') lines.push(`N° ${val}`)
          else lines.push(val)
        } else if (typeof val === 'number' && key === 'number') { lines.push(`N° ${val}`) }
        else if (Array.isArray(val) || (val && typeof val === 'object')) lines.push(...extractTextFromJson(val, depth + 1))
      } else { lines.push(...extractTextFromJson(val, depth + 1)) }
    }
  }
  return lines
}

export function formatJournalData(raw: string): string {
  try {
    const parsed = JSON.parse(raw)
    const source = parsed?.text !== undefined ? parsed.text : parsed
    if (typeof source === 'string') return source
    const lines = extractTextFromJson(source)
    const seen = new Set<string>()
    const unique = lines.filter(l => { const t = l.trim(); if (t.length < 2 || seen.has(t)) return false; seen.add(t); return true })
    return unique.length > 0 ? unique.join('\n') : raw
  } catch { return raw }
}

export type SecurityAction = { kind: 'block' | 'tokenize' | 'alias' | 'mask'; field: string; real_value: string; replaced_by: string }

export function parseSecurityActions(raw?: string): SecurityAction[] {
  if (!raw) return []
  try { const parsed = JSON.parse(raw); if (Array.isArray(parsed?.security)) return parsed.security as SecurityAction[] } catch { /* ignore */ }
  return []
}

const HIGHLIGHT_PATTERNS = [{ pattern: '[bloqué]', display: undefined as string | undefined, color: '#dc2626', bg: '#fee2e2' }]

export function HighlightedText({ text, actions }: { text: string; actions: SecurityAction[] }) {
  const patterns: { pattern: string; display?: string; color: string; bg: string }[] = [...HIGHLIGHT_PATTERNS]
  const dbTokenActions = actions.filter(a => a.kind === 'tokenize' && a.replaced_by.startsWith('tok_'))
  dbTokenActions.forEach((a, i) => { if (!patterns.find(p => p.pattern === a.replaced_by)) patterns.push({ pattern: a.replaced_by, display: `${a.replaced_by} #${i + 1}`, color: '#d97706', bg: '#fef3c7' }) })
  for (const a of actions) {
    if (!a.replaced_by || patterns.find(p => p.pattern === a.replaced_by)) continue
    if (a.kind === 'alias')    patterns.push({ pattern: a.replaced_by, color: '#059669', bg: '#d1fae5' })
    if (a.kind === 'mask')     patterns.push({ pattern: a.replaced_by, color: '#dc2626', bg: '#fee2e2' })
    if (a.kind === 'tokenize') patterns.push({ pattern: a.replaced_by, color: '#d97706', bg: '#fef3c7' })
  }
  const parts: React.ReactNode[] = []
  let remaining = text; let key = 0
  while (remaining.length > 0) {
    let earliest = -1; let matchedPattern: typeof patterns[0] | null = null
    for (const p of patterns) { const idx = remaining.indexOf(p.pattern); if (idx !== -1 && (earliest === -1 || idx < earliest)) { earliest = idx; matchedPattern = p } }
    if (earliest === -1 || !matchedPattern) { parts.push(remaining); break }
    if (earliest > 0) parts.push(remaining.slice(0, earliest))
    parts.push(<span key={key++} style={{ fontWeight: 700, color: matchedPattern.color, background: matchedPattern.bg, borderRadius: 3, padding: '0 2px' }}>{matchedPattern.display ?? matchedPattern.pattern}</span>)
    remaining = remaining.slice(earliest + matchedPattern.pattern.length)
  }
  return <>{parts}</>
}

export function JournalEntryRow({ entry }: {
  entry: { ts: number; tool: string; query: string; results: number; blocked: boolean; data?: string | Record<string, unknown> }
}) {
  const [expanded, setExpanded] = useState(false)
  const date = new Date(entry.ts * 1000)
  const time = date.toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' })
  const day  = date.toLocaleDateString('fr-FR', { day: '2-digit', month: '2-digit' })
  const rawData = entry.data == null ? undefined : typeof entry.data === 'string' ? entry.data : JSON.stringify(entry.data)
  const secActions = parseSecurityActions(rawData)
  const displayData = rawData ? formatJournalData(rawData) : ''
  const hasContent = !!rawData && !entry.blocked
  const color = connectorColor(entry.tool)
  const { connector, action } = parseToolDisplay(entry.tool)

  return (
    <JournalRow $blocked={entry.blocked} $color={color} onClick={hasContent ? () => setExpanded(v => !v) : undefined} style={hasContent ? { cursor: 'pointer' } : undefined}>
      <JournalMeta>
        <JournalTime>{day} {time}</JournalTime>
        <JournalConnector $color={entry.blocked ? '#dc2626' : color}>{connector}</JournalConnector>
        {action && <JournalAction>{action}</JournalAction>}
        <JournalResultsBadge $blocked={entry.blocked}>
          {entry.blocked ? '⛔ bloqué' : `${entry.results} résultat${entry.results !== 1 ? 's' : ''}`}
        </JournalResultsBadge>
        {secActions.length > 0 && (
          <>
            {secActions.some(a => a.kind === 'tokenize') && <SecurityBadge $kind="tokenize">🔑 {secActions.filter(a => a.kind === 'tokenize').length} tokenisé</SecurityBadge>}
            {secActions.some(a => a.kind === 'alias') && <SecurityBadge $kind="alias">👤 alias</SecurityBadge>}
            {secActions.some(a => a.kind === 'mask') && <SecurityBadge $kind="mask">🔒 {secActions.filter(a => a.kind === 'mask').length} masqué</SecurityBadge>}
            {secActions.some(a => a.kind === 'block') && <SecurityBadge $kind="block">🚫 bloqué</SecurityBadge>}
          </>
        )}
      </JournalMeta>
      {entry.query && <JournalQuery>{entry.query}</JournalQuery>}
      {expanded && displayData && (<DataBox><HighlightedText text={displayData} actions={secActions} /></DataBox>)}
      {expanded && secActions.length > 0 && (
        <SecurityBox>
          <SecurityBoxHeader><Shield size={11} />Filtres de confidentialité appliqués</SecurityBoxHeader>
          {(['[email masqué', '[téléphone masqué'] as const).map(prefix => {
            const items = secActions.filter(a => a.kind === 'mask' && a.replaced_by.startsWith(prefix))
            if (items.length === 0) return null
            const label = prefix === '[email masqué' ? 'Emails masqués' : 'Téléphones masqués'
            return (
              <SecurityGroup key={prefix}>
                <SecurityGroupTitle $color="#dc2626"><ShieldOff size={11} />{label}</SecurityGroupTitle>
                <SecurityMaskList>{items.map((a, i) => { const numMatch = a.replaced_by.match(/#(\d+)\]$/); const num = numMatch ? `#${numMatch[1]}` : `#${i + 1}`; return (<SecurityMaskItem key={i}><SecurityMaskNum>{num}</SecurityMaskNum><SecurityMaskEmail>{a.real_value}</SecurityMaskEmail></SecurityMaskItem>) })}</SecurityMaskList>
              </SecurityGroup>
            )
          })}
          {([{ prefix: '[TOKEN masqué', label: 'Tokens détectés' }, { prefix: '[CLÉ API masquée', label: 'Clés API masquées' }, { prefix: '[DB masquée', label: 'Connexions DB masquées' }] as const).map(({ prefix, label }) => {
            const items = secActions.filter(a => a.kind === 'tokenize' && a.replaced_by.startsWith(prefix))
            if (items.length === 0) return null
            return (
              <SecurityGroup key={prefix}>
                <SecurityGroupTitle $color="#d97706"><Key size={11} />{label}</SecurityGroupTitle>
                <SecurityMaskList>{items.map((a, i) => { const numMatch = a.replaced_by.match(/#(\d+)\]$/); const num = numMatch ? `#${numMatch[1]}` : `#${i + 1}`; return (<SecurityMaskItem key={i}><SecurityMaskNum>{num}</SecurityMaskNum><SecurityMaskEmail>{a.real_value}</SecurityMaskEmail></SecurityMaskItem>) })}</SecurityMaskList>
              </SecurityGroup>
            )
          })}
          {secActions.some(a => a.kind === 'tokenize' && a.replaced_by.startsWith('tok_')) && (
            <SecurityGroup>
              <SecurityGroupTitle $color="#d97706"><Key size={11} />Données tokenisées</SecurityGroupTitle>
              <SecurityMaskList>{secActions.filter(a => a.kind === 'tokenize' && a.replaced_by.startsWith('tok_')).map((a, i) => (<SecurityRow key={i}><SecurityMaskNum>#{i + 1}</SecurityMaskNum><SecurityRealValue>{a.field ? `${a.field}: ` : ''}{a.real_value}</SecurityRealValue><SecurityArrow>→</SecurityArrow><SecurityReplaced $kind="tokenize">{a.replaced_by}</SecurityReplaced></SecurityRow>))}</SecurityMaskList>
            </SecurityGroup>
          )}
          {secActions.some(a => a.kind === 'alias') && (
            <SecurityGroup>
              <SecurityGroupTitle $color="#059669"><UserRound size={11} />Alias appliqués</SecurityGroupTitle>
              <SecurityMaskList>{secActions.filter(a => a.kind === 'alias').map((a, i) => (<SecurityRow key={i}><SecurityRealValue>{a.real_value}</SecurityRealValue><SecurityArrow>→</SecurityArrow><SecurityReplaced $kind="alias">{a.replaced_by}</SecurityReplaced></SecurityRow>))}</SecurityMaskList>
            </SecurityGroup>
          )}
          {secActions.some(a => a.kind === 'block') && (
            <SecurityGroup>
              <SecurityGroupTitle $color="#dc2626"><ShieldOff size={11} />Valeurs bloquées</SecurityGroupTitle>
              <SecurityMaskList>{secActions.filter(a => a.kind === 'block').map((a, i) => (<SecurityRow key={i}><SecurityRealValue>{a.real_value}</SecurityRealValue><SecurityArrow>→</SecurityArrow><SecurityReplaced $kind="block">{a.replaced_by}</SecurityReplaced></SecurityRow>))}</SecurityMaskList>
            </SecurityGroup>
          )}
        </SecurityBox>
      )}
    </JournalRow>
  )
}

// ─── Carte d'action (EN ATTENTE + HISTORIQUE) ────────────────────────────────

const ActionCard = styled.div<{ $status: string }>`
  background: #fff; border: 1px solid #e5e7eb; border-radius: 12px; padding: 16px 18px;
  box-shadow: 0 1px 4px rgba(0,0,0,.04); position: relative; transition: box-shadow .15s;
  &:hover { box-shadow: 0 2px 8px rgba(0,0,0,.07); }
`
const CardTop = styled.div`display: flex; align-items: center; margin-bottom: 10px; gap: 8px; padding-right: 70px;`
const CardTopLeft = styled.div`display: flex; align-items: center; gap: 8px; min-width: 0; flex: 1;`
const SourceBadge = styled.span`font-size: 11px; font-weight: 700; letter-spacing: .04em; padding: 3px 8px; border-radius: 5px; background: #ededff; color: #5b5ef4; white-space: nowrap; flex-shrink: 0;`
const ToolName = styled.span`font-size: 13px; font-weight: 600; color: #1a1d23; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;`
const StatusPill = styled.span<{ $status: string }>`
  position: absolute; top: 12px; right: 14px;
  font-size: 10px; font-weight: 600; padding: 2px 8px; border-radius: 99px; white-space: nowrap;
  background: ${({ $status }) => $status === 'pending' ? '#fef3c7' : $status === 'approved' ? '#d1fae5' : $status === 'rejected' ? '#fee2e2' : '#f3f4f6'};
  color: ${({ $status }) => $status === 'pending' ? '#92400e' : $status === 'approved' ? '#065f46' : $status === 'rejected' ? '#991b1b' : '#6b7280'};
`
export const StatusLabel: Record<string, string> = { pending: 'En attente', approved: 'Approuvée', rejected: 'Refusée', expired: 'Expirée' }
const ParamsGrid = styled.div`background: #f8fafc; border: 1px solid #e8eaed; border-radius: 8px; padding: 10px 14px; display: flex; flex-direction: column; gap: 4px;`
const ParamRow = styled.div`display: flex; gap: 8px; align-items: baseline; min-width: 0;`
const ParamKey = styled.span`font-size: 11px; font-weight: 600; color: #9ca3af; white-space: nowrap; text-transform: lowercase; font-family: 'SF Mono', monospace; flex-shrink: 0;`
const ParamVal = styled.span`font-size: 12px; color: #374151; word-break: break-word; line-height: 1.5;`
const PreviewBox = styled.pre`background: #f8fafc; border: 1px solid #e8eaed; border-radius: 8px; padding: 10px 14px; font-size: 12px; color: #374151; line-height: 1.6; white-space: pre-wrap; word-break: break-word; margin: 0; font-family: inherit;`
const CardFooter = styled.div`display: flex; align-items: center; justify-content: space-between; margin-top: 10px;`
const CardDate = styled.span`font-size: 11px; color: #9ca3af;`
const TimerBadge = styled.span<{ $urgent: boolean }>`font-size: 11px; font-weight: 600; color: ${({ $urgent }) => $urgent ? '#dc2626' : '#9ca3af'};`
export const ActionBtns = styled.div`display: flex; gap: 8px; margin-top: 12px;`
export const ApproveBtn = styled.button`
  flex: 1; padding: 9px 14px; border-radius: 8px; font-size: 13px; font-weight: 600;
  border: none; background: #10b981; color: #fff; cursor: pointer; transition: background .15s;
  display: flex; align-items: center; justify-content: center; gap: 5px;
  &:hover { background: #059669; } &:disabled { opacity: .5; cursor: not-allowed; }
`
export const RejectBtn = styled.button`
  padding: 9px 18px; border-radius: 8px; font-size: 13px; font-weight: 600;
  border: 1px solid #e5e7eb; background: #fff; color: #6b7280; cursor: pointer; transition: all .15s;
  display: flex; align-items: center; gap: 5px;
  &:hover { border-color: #fca5a5; color: #dc2626; background: #fef2f2; } &:disabled { opacity: .5; cursor: not-allowed; }
`
const ExecResult = styled.div<{ $ok: boolean }>`
  margin-top: 12px; padding: 10px 14px; border-radius: 8px; font-size: 12px; font-weight: 500;
  background: ${({ $ok }) => $ok ? '#d1fae5' : '#fee2e2'};
  color: ${({ $ok }) => $ok ? '#065f46' : '#991b1b'};
`

export function ActionCardItem({
  action,
  onDecision,
  onApprove,
  onReject,
}: {
  action: ActionRequest
  onDecision: () => void
  onApprove?: (id: string) => Promise<ActionRequest>
  onReject?: (id: string) => Promise<ActionRequest>
}) {
  const [loading, setLoading] = useState(false)
  const [now, setNow] = useState(() => Math.floor(Date.now() / 1000))

  useEffect(() => {
    if (action.status !== 'pending') return
    const t = setInterval(() => setNow(Math.floor(Date.now() / 1000)), 1000)
    return () => clearInterval(t)
  }, [action.status])

  async function approve() {
    if (!onApprove) return
    setLoading(true)
    try { await onApprove(action.id) } finally { setLoading(false); onDecision() }
  }
  async function reject() {
    if (!onReject) return
    setLoading(true)
    try { await onReject(action.id) } finally { setLoading(false); onDecision() }
  }

  const rawTool = action.tool
  let source = ''; let toolDisplay = ''
  if (rawTool.includes(':')) {
    const [s, t] = rawTool.split(':', 2)
    source = s.charAt(0).toUpperCase() + s.slice(1)
    toolDisplay = t.replace(/_/g, ' ')
  } else if (rawTool.startsWith('act_')) {
    toolDisplay = rawTool.replace('act_', '').replace(/_/g, ' ')
  } else {
    const parts = rawTool.split('_')
    source = parts[0].charAt(0).toUpperCase() + parts[0].slice(1)
    toolDisplay = parts.slice(1).join(' ')
  }

  const params = action.params as Record<string, unknown>
  const paramEntries = Object.entries(params).filter(([, v]) => v !== undefined && v !== '' && v !== null)

  const date = new Date(action.created_at * 1000).toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' })
  const dateDay = new Date(action.created_at * 1000).toLocaleDateString('fr-FR', { day: '2-digit', month: '2-digit' })
  const expiresIn = Math.max(0, action.expires_at - now)
  const urgent = expiresIn < 60

  return (
    <ActionCard $status={action.status}>
      <CardTop>
        <CardTopLeft>
          {source && <SourceBadge>{source}</SourceBadge>}
          <ToolName>{toolDisplay || rawTool}</ToolName>
        </CardTopLeft>
        <StatusPill $status={action.status}>{StatusLabel[action.status] ?? action.status}</StatusPill>
      </CardTop>

      {paramEntries.length > 0 ? (
        <ParamsGrid>
          {paramEntries.map(([k, v]) => (
            <ParamRow key={k}>
              <ParamKey>{k}</ParamKey>
              <ParamVal>{typeof v === 'object' ? JSON.stringify(v) : String(v)}</ParamVal>
            </ParamRow>
          ))}
        </ParamsGrid>
      ) : (
        <PreviewBox>{action.preview}</PreviewBox>
      )}

      <CardFooter>
        <CardDate>{dateDay} à {date}</CardDate>
        {action.status === 'pending' && expiresIn > 0 && (
          <TimerBadge $urgent={urgent}>{urgent ? `⚠ ${expiresIn}s` : `${Math.ceil(expiresIn / 60)} min`}</TimerBadge>
        )}
      </CardFooter>

      {action.execution_result && (
        <ExecResult $ok={action.execution_result.startsWith('ok:')}>
          {action.execution_result.startsWith('ok:') ? '✓ ' : '✕ '}
          {action.execution_result.replace(/^(ok|err): /, '')}
        </ExecResult>
      )}

      {action.status === 'pending' && (onApprove || onReject) && (
        <ActionBtns>
          {onApprove && <ApproveBtn onClick={approve} disabled={loading}><span>✓</span> Approuver</ApproveBtn>}
          {onReject  && <RejectBtn  onClick={reject}  disabled={loading}><span>✕</span> Rejeter</RejectBtn>}
        </ActionBtns>
      )}
    </ActionCard>
  )
}
