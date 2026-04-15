import { useState, useEffect } from 'react'
import styled from 'styled-components'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { api } from '../api'
import type { PeerResponse, QueryHistoryEntry, ToolAccessMode, ActionRequest } from '../api'
import { icons } from '../lib/assets'
import {
  CardList, EmptyMsg, JournalList, JournalEntryRow, ActionCardItem,
} from '../components/ActionFlux'

// ─── Constantes ──────────────────────────────────────────────────────────────



// ─── Styles de base ──────────────────────────────────────────────────────────

const Page = styled.div`
  display: flex;
  flex-direction: column;
  gap: 28px;
`

const PageHeader = styled.div`
  display: flex;
  align-items: center;
  justify-content: space-between;
`

const PageTitle = styled.h1`
  font-size: 22px;
  font-weight: 700;
  color: #1a1d23;
  letter-spacing: -.02em;
`

const SectionTitle = styled.h2`
  font-size: 11px;
  font-weight: 600;
  color: #9ca3af;
  text-transform: uppercase;
  letter-spacing: .08em;
  margin-bottom: -12px;
`

const Card = styled.div`
  background: #fff;
  border: 1px solid #e8eaed;
  border-radius: 16px;
  overflow: hidden;
  box-shadow: 0 1px 3px rgba(0,0,0,.04);
`

const CardHeader = styled.div`
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 18px 22px 16px;
  border-bottom: 1px solid #f3f4f6;
`

const CardTitle = styled.span`
  font-size: 13px;
  font-weight: 600;
  color: #1a1d23;
`

const CardBody = styled.div`
  padding: 20px 22px 22px;
  display: flex;
  flex-direction: column;
  gap: 14px;
`

// ─── Identité ────────────────────────────────────────────────────────────────

const IdentityCard = styled(Card)`
  background: linear-gradient(135deg, #5b5ef4 0%, #7c3aed 100%);
  border: none;
  color: #fff;
`

const IdentityInner = styled.div`
  padding: 24px;
  display: flex;
  flex-direction: column;
  gap: 18px;
`

const IdentityTop = styled.div`
  display: flex;
  align-items: center;
  gap: 14px;
`

const IdentityAvatar = styled.div`
  width: 48px;
  height: 48px;
  border-radius: 50%;
  background: rgba(255,255,255,.2);
  border: 2px solid rgba(255,255,255,.35);
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 800;
  font-size: 17px;
  color: #fff;
  flex-shrink: 0;
`

const IdentityName = styled.p`
  font-size: 17px;
  font-weight: 700;
  color: #fff;
`

const IdentityId = styled.p`
  font-size: 11px;
  color: rgba(255,255,255,.6);
  font-family: 'SF Mono', 'Fira Code', monospace;
  margin-top: 2px;
`

const IdentityOnline = styled.span`
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: 11px;
  font-weight: 500;
  color: rgba(255,255,255,.8);
  margin-left: auto;

  &::before {
    content: '';
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: #34d399;
    box-shadow: 0 0 0 2px rgba(52,211,153,.3);
  }
`

const InviteLinkBox = styled.div`
  background: rgba(255,255,255,.12);
  border: 1px solid rgba(255,255,255,.2);
  border-radius: 10px;
  padding: 12px 14px;
  display: flex;
  align-items: center;
  gap: 10px;
`

const InviteLinkText = styled.code`
  font-size: 11px;
  color: rgba(255,255,255,.85);
  word-break: break-all;
  flex: 1;
  line-height: 1.5;
`

const InvitePlaceholder = styled.p`
  font-size: 12px;
  color: rgba(255,255,255,.5);
  flex: 1;
  font-style: italic;
`

const WhiteBtn = styled.button<{ $sm?: boolean }>`
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 7px;
  padding: ${({ $sm }) => $sm ? '7px 10px' : '10px 18px'};
  background: rgba(255,255,255,.18);
  color: #fff;
  border: 1px solid rgba(255,255,255,.3);
  border-radius: 9px;
  font-size: ${({ $sm }) => $sm ? '12px' : '13px'};
  font-weight: 500;
  font-family: inherit;
  cursor: pointer;
  transition: all .15s;
  white-space: nowrap;
  flex-shrink: 0;

  &:hover { background: rgba(255,255,255,.28); }
  &:disabled { opacity: .4; cursor: not-allowed; }
`

// ─── Peers ───────────────────────────────────────────────────────────────────

const PeerCard = styled.div<{ $selected?: boolean }>`
  background: #fff;
  border: ${({ $selected }) => $selected ? '2px solid #5b5ef4' : '1px solid #e8eaed'};
  border-radius: 16px;
  overflow: hidden;
  box-shadow: ${({ $selected }) => $selected ? '0 0 0 3px rgba(91,94,244,.15)' : '0 1px 3px rgba(0,0,0,.04)'};
  cursor: ${({ $selected }) => $selected !== undefined ? 'pointer' : 'default'};
  transition: border-color .15s, box-shadow .15s;
`

const PeerHeader = styled.div`
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px;
`

const PeerLeft = styled.div`
  display: flex;
  align-items: center;
  gap: 12px;
`

const PeerAvatar = styled.div<{ $connected: boolean }>`
  width: 40px;
  height: 40px;
  border-radius: 50%;
  background: ${({ $connected }) => $connected ? '#ede9fe' : '#f3f4f6'};
  color: ${({ $connected }) => $connected ? '#6d28d9' : '#9ca3af'};
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: 14px;
  flex-shrink: 0;
`

const PeerName = styled.p`
  font-size: 14px;
  font-weight: 600;
  color: #1a1d23;
`

const PeerMeta = styled.p`
  font-size: 11px;
  color: #9ca3af;
  margin-top: 2px;
`

const PeerRight = styled.div`
  display: flex;
  align-items: center;
  gap: 8px;
`

const StatusDot = styled.span<{ $on: boolean }>`
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: 11px;
  font-weight: 500;
  color: ${({ $on }) => $on ? '#059669' : '#9ca3af'};

  &::before {
    content: '';
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: ${({ $on }) => $on ? '#10b981' : '#d1d5db'};
  }
`

const IconBtn = styled.button<{ $danger?: boolean; $active?: boolean }>`
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 8px;
  border: 1px solid ${({ $active }) => $active ? '#c7d2fe' : '#e5e7eb'};
  background: ${({ $active }) => $active ? '#eef2ff' : '#fff'};
  color: ${({ $danger, $active }) => $danger ? '#ef4444' : $active ? '#5b5ef4' : '#6b7280'};
  cursor: pointer;
  transition: all .15s;

  &:hover {
    background: ${({ $danger, $active }) => $danger ? '#fee2e2' : $active ? '#e0e7ff' : '#f3f4f6'};
    border-color: ${({ $danger, $active }) => $danger ? '#fca5a5' : $active ? '#a5b4fc' : '#d1d5db'};
  }
  &:disabled { opacity: .4; cursor: not-allowed; }
`

// ─── Form elements ────────────────────────────────────────────────────────────

const FormRow = styled.div`
  display: flex;
  flex-direction: column;
  gap: 6px;
`

const Label = styled.label`
  font-size: 12px;
  font-weight: 500;
  color: #374151;
`

const Input = styled.input`
  width: 100%;
  font-size: 14px;
  font-family: inherit;
  padding: 10px 14px;
  background: #fff;
  border: 1px solid #e5e7eb;
  border-radius: 10px;
  color: #1a1d23;
  outline: none;
  box-sizing: border-box;

  &:focus {
    border-color: #5b5ef4;
    box-shadow: 0 0 0 3px rgba(91,94,244,.12);
  }
  &::placeholder { color: #9ca3af; }
`

const Hint = styled.p`
  font-size: 11px;
  color: #9ca3af;
`

const PrimaryBtn = styled.button`
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 10px 20px;
  background: #5b5ef4;
  color: #fff;
  border: none;
  border-radius: 10px;
  font-size: 13px;
  font-weight: 500;
  font-family: inherit;
  cursor: pointer;
  transition: background .15s;

  &:hover { background: #4a4de3; }
  &:disabled { opacity: .4; cursor: not-allowed; }
`

const Alert = styled.div<{ $variant: 'success' | 'error' }>`
  font-size: 13px;
  border-radius: 8px;
  padding: 10px 14px;
  background: ${({ $variant }) => $variant === 'success' ? '#d1fae5' : '#fee2e2'};
  color: ${({ $variant }) => $variant === 'success' ? '#065f46' : '#991b1b'};
`

// ─── Sous-composant : identité locale ────────────────────────────────────────

function IdentitySection() {
  const [link, setLink] = useState('')
  const [copied, setCopied] = useState(false)

  const { data: identity } = useQuery({
    queryKey: ['my-identity'],
    queryFn: api.getMyIdentity,
  })

  const generateMut = useMutation({
    mutationFn: api.generateInvite,
    onSuccess: (data) => setLink(data.link),
  })

  const copy = () => {
    if (!link) return
    navigator.clipboard.writeText(link)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  const initials = identity?.display_name?.slice(0, 2).toUpperCase() ?? '??'
  const shortId = identity?.peer_id ? `${identity.peer_id.slice(0, 8)}…${identity.peer_id.slice(-6)}` : '...'

  return (
    <IdentityCard>
      <IdentityInner>
        <IdentityTop>
          <IdentityAvatar>{initials}</IdentityAvatar>
          <div style={{ flex: 1, minWidth: 0 }}>
            <IdentityName>{identity?.display_name ?? '...'}</IdentityName>
            <IdentityId>{shortId}</IdentityId>
          </div>
          <IdentityOnline>En ligne</IdentityOnline>
        </IdentityTop>

        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
          <div style={{ fontSize: 11, color: 'rgba(255,255,255,.6)', fontWeight: 500 }}>
            Partager mon lien d'accès
          </div>
          <InviteLinkBox>
            {link
              ? <InviteLinkText>{link}</InviteLinkText>
              : <InvitePlaceholder>Cliquer sur "Générer" pour créer un lien sécurisé</InvitePlaceholder>
            }
            <div style={{ display: 'flex', gap: 6, flexShrink: 0 }}>
              <WhiteBtn $sm onClick={() => generateMut.mutate()} disabled={generateMut.isPending}>
                <icons.Link size={12} />
                {generateMut.isPending ? '...' : 'Générer'}
              </WhiteBtn>
              {link && (
                <WhiteBtn $sm onClick={copy}>
                  {copied ? <icons.Check size={12} /> : <icons.Copy size={12} />}
                </WhiteBtn>
              )}
            </div>
          </InviteLinkBox>
          {link && (
            <p style={{ fontSize: 11, color: 'rgba(255,255,255,.5)' }}>
              Envoyez ce lien à votre collègue. Il fonctionne sur tous les réseaux (WiFi, 4G…) sans configuration.
            </p>
          )}
        </div>
      </IdentityInner>
    </IdentityCard>
  )
}

// ─── Labels lisibles pour chaque tool/connecteur ─────────────────────────────

// Uniquement les connecteurs cloud — les outils locaux (notes, fichiers, iMessage…)
// ne sont pas partagés via P2P.
const TOOL_LABELS: { id: string; label: string }[] = [
  { id: 'github',     label: 'GitHub'       },
  { id: 'notion',     label: 'Notion'       },
  { id: 'slack',      label: 'Slack'        },
  { id: 'linear',     label: 'Linear'       },
  { id: 'jira',       label: 'Jira'         },
  { id: 'gitlab',     label: 'GitLab'       },
  { id: 'supabase',   label: 'Supabase'     },
  { id: 'vercel',     label: 'Vercel'       },
  { id: 'railway',    label: 'Railway'      },
  { id: 'render',     label: 'Render'       },
  { id: 'stripe',     label: 'Stripe'       },
  { id: 'hubspot',    label: 'HubSpot'      },
  { id: 'discord',    label: 'Discord'      },
  { id: 'resend',     label: 'Resend'       },
  { id: 'twilio',     label: 'Twilio'       },
  { id: 'figma',      label: 'Figma'        },
  { id: 'posthog',    label: 'PostHog'      },
  { id: 'sentry',     label: 'Sentry'       },
  { id: 'cloudflare', label: 'Cloudflare'   },
  { id: 'gcal',       label: 'Google Cal'   },
  { id: 'gmail',      label: 'Gmail'        },
  { id: 'calendly',   label: 'Calendly'     },
  { id: 'n8n',        label: 'n8n'          },
  { id: 'shopify',    label: 'Shopify'      },
  { id: 'reddit',     label: 'Reddit'       },
  { id: 'trello',     label: 'Trello'       },
  { id: 'todoist',    label: 'Todoist'      },
  { id: 'airtable',   label: 'Airtable'     },
  { id: 'browser',    label: 'Browser'      },
]

// Aucun tool local de base — uniquement les connecteurs cloud configurés
const BASE_TOOLS: string[] = []

const PeerSection = styled.div`
  padding: 16px 20px 18px;
  border-top: 1px solid #f3f4f6;
`

const PeerSectionTitle = styled.p`
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: .06em;
  color: #9ca3af;
  margin-bottom: 10px;
`

// Ordre du cycle : désactivé → auto → approbation → désactivé
const MODE_CYCLE: ToolAccessMode[] = ['disabled', 'auto', 'require']

const MODE_STYLE: Record<ToolAccessMode, { bg: string; border: string; color: string; dot: string; label: string }> = {
  disabled: { bg: '#f9fafb', border: '#e5e7eb', color: '#9ca3af', dot: '#d1d5db', label: 'Désactivé' },
  auto:     { bg: '#d1fae5', border: '#a7f3d0', color: '#065f46', dot: '#10b981', label: 'Auto' },
  require:  { bg: '#fef3c7', border: '#fde68a', color: '#92400e', dot: '#f59e0b', label: 'Approbation' },
}

const ToolChip = styled.button<{ $mode: ToolAccessMode; $readonly?: boolean }>`
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: 12px;
  font-weight: 500;
  padding: 5px 11px;
  border-radius: 20px;
  border: 1px solid ${({ $mode }) => MODE_STYLE[$mode].border};
  background: ${({ $mode }) => MODE_STYLE[$mode].bg};
  color: ${({ $mode }) => MODE_STYLE[$mode].color};
  cursor: ${({ $readonly }) => $readonly ? 'default' : 'pointer'};
  transition: all .15s;
  font-family: inherit;
  user-select: none;
  &:hover { opacity: ${({ $readonly }) => $readonly ? 1 : 0.78}; }

  &::before {
    content: '';
    width: 6px; height: 6px;
    border-radius: 50%;
    background: ${({ $mode }) => MODE_STYLE[$mode].dot};
    flex-shrink: 0;
  }
`

const ToolChipName = styled.span`font-weight: 600;`
const ToolChipMode = styled.span`font-size: 10px; opacity: .75;`

const ToolGrid = styled.div`
  display: flex;
  flex-wrap: wrap;
  gap: 7px;
`

const Legend = styled.div`
  display: flex;
  gap: 14px;
  flex-wrap: wrap;
  margin-top: 10px;
  font-size: 11px;
  color: #6b7280;
`

// ─── Card peer complète ───────────────────────────────────────────────────────

// Bouton "Sélectionner" / "Sélectionné"
const SelectBtn = styled.button<{ $active: boolean }>`
  display: inline-flex; align-items: center; gap: 6px;
  padding: 6px 14px; border-radius: 8px; font-size: 12px; font-weight: 600;
  font-family: inherit; cursor: pointer; transition: all .15s; white-space: nowrap;
  border: 1px solid ${({ $active }) => $active ? '#5b5ef4' : '#e5e7eb'};
  background: ${({ $active }) => $active ? '#5b5ef4' : '#fff'};
  color: ${({ $active }) => $active ? '#fff' : '#6b7280'};
  &:hover { background: ${({ $active }) => $active ? '#4a4de3' : '#f3f4f6'}; }
`

// Bouton chevron expand/collapse
const ChevronBtn = styled.button`
  display: flex; align-items: center; justify-content: center;
  width: 30px; height: 30px; border-radius: 8px; border: 1px solid #e5e7eb;
  background: #fff; color: #9ca3af; cursor: pointer; transition: all .15s; flex-shrink: 0;
  &:hover { background: #f3f4f6; color: #6b7280; }
`

function PeerCardItem({ peer, selected, onSelect }: {
  peer: PeerResponse; selected?: boolean; onSelect?: () => void
}) {
  const qc = useQueryClient()
  const [expanded, setExpanded] = useState(false)

  // Connecteurs configurés sur MON Mac
  const { data: configured = [] } = useQuery({
    queryKey: ['configured-connectors'],
    queryFn: api.getConfiguredConnectors,
    staleTime: 60_000,
  })

  // Permissions actuellement en vigueur (serveur)
  const { data: savedPerms = {} } = useQuery({
    queryKey: ['peer-tool-permissions', peer.peer_id],
    queryFn: () => api.getPeerToolPermissions(peer.peer_id),
  })

  const [draft, setDraft] = useState<Record<string, ToolAccessMode> | null>(null)

  const displayed = draft ?? savedPerms
  const isDirty = draft !== null

  const permsMut = useMutation({
    mutationFn: (p: Record<string, ToolAccessMode>) =>
      api.setPeerToolPermissions(peer.peer_id, p),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['peer-tool-permissions', peer.peer_id] })
      setDraft(null)
    },
  })

  const deleteMut = useMutation({
    mutationFn: () => api.deletePeer(peer.peer_id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['network-peers'] }),
  })

  const myToolIds = [...BASE_TOOLS, ...configured]

  const cycleMode = (id: string) => {
    const current = (draft ?? savedPerms)[id] as ToolAccessMode | undefined ?? 'disabled'
    const idx = MODE_CYCLE.indexOf(current)
    const next = MODE_CYCLE[(idx + 1) % MODE_CYCLE.length]
    setDraft(prev => ({ ...(prev ?? savedPerms), [id]: next }))
  }

  // Construit l'état complet explicite (tous les tools, disabled par défaut)
  const buildFullState = (): Record<string, ToolAccessMode> =>
    myToolIds.reduce((acc, id) => {
      acc[id] = ((draft ?? savedPerms)[id] as ToolAccessMode | undefined) ?? 'disabled'
      return acc
    }, {} as Record<string, ToolAccessMode>)

  const applyChanges = () => {
    permsMut.mutate(buildFullState())
  }

  const cancelChanges = () => setDraft(null)

  const labelFor = (id: string) =>
    TOOL_LABELS.find(t => t.id === id)?.label ?? id

  const initials = peer.display_name.slice(0, 2).toUpperCase()
  const lastSeen = peer.last_seen
    ? new Date(peer.last_seen * 1000).toLocaleDateString('fr-FR', {
        day: '2-digit', month: '2-digit', hour: '2-digit', minute: '2-digit',
      })
    : null

  // Ce que le peer nous autorise à utiliser sur son Mac (reçu via PermissionsSync)
  const { data: grantedPerms = {} } = useQuery({
    queryKey: ['peer-granted-permissions', peer.peer_id],
    queryFn: () => api.getPeerGrantedPermissions(peer.peer_id),
  })

  // SSE — invalidation instantanée dès que le peer envoie un PermissionsSync
  useEffect(() => {
    const es = new EventSource('/api/network/stream')
    es.addEventListener('permissions_updated', (e: MessageEvent) => {
      if (e.data === peer.peer_id) {
        qc.invalidateQueries({ queryKey: ['peer-granted-permissions', peer.peer_id] })
      }
    })
    return () => es.close()
  }, [peer.peer_id, qc])
  const theirTools = Object.entries(grantedPerms)
    .filter(([, mode]) => mode !== 'disabled')
    .map(([id, mode]) => ({ id, mode: mode as ToolAccessMode }))

  return (
    <PeerCard $selected={selected}>
      {/* ── Header ── */}
      <PeerHeader
        onClick={() => setExpanded(v => !v)}
        style={{ cursor: 'pointer', userSelect: 'none' }}
      >
        <PeerLeft>
          <PeerAvatar $connected={peer.connected}>{initials}</PeerAvatar>
          <div>
            <PeerName>{peer.display_name}</PeerName>
            <PeerMeta>
              {peer.peer_id.slice(0, 8)}…
              {lastSeen && ` · vu le ${lastSeen}`}
            </PeerMeta>
          </div>
        </PeerLeft>
        <PeerRight onClick={e => e.stopPropagation()}>
          <StatusDot $on={peer.connected}>
            {peer.connected ? 'Connecté' : 'Hors ligne'}
          </StatusDot>
          <SelectBtn
            $active={!!selected}
            onClick={() => onSelect?.()}
          >
            {selected ? <><icons.Check size={12} />Sélectionné</> : 'Sélectionner'}
          </SelectBtn>
          <IconBtn $danger onClick={() => deleteMut.mutate()} disabled={deleteMut.isPending} title="Retirer">
              <icons.UserX size={13} />
            </IconBtn>
          <ChevronBtn onClick={() => setExpanded(v => !v)} title={expanded ? 'Réduire' : 'Développer'}>
            {expanded ? <icons.ChevronUp size={13} /> : <icons.ChevronDown size={13} />}
          </ChevronBtn>
        </PeerRight>
      </PeerHeader>

      {/* ── Corps : visible seulement si expanded ── */}
      {expanded && (
        <>
          {/* ── Section 1 : ce que j'autorise mon ami à utiliser ── */}
          <PeerSection>
            <PeerSectionTitle>
              Ce que j'autorise {peer.display_name} à utiliser sur mon Mac
            </PeerSectionTitle>
            <ToolGrid>
              {myToolIds.map(id => {
                const mode = (displayed[id] as ToolAccessMode | undefined) ?? 'disabled'
                return (
                  <ToolChip
                    key={id}
                    $mode={mode}
                    onClick={() => cycleMode(id)}
                    title="Cliquer pour changer : Désactivé → Auto → Approbation"
                  >
                    <ToolChipName>{labelFor(id)}</ToolChipName>
                    <ToolChipMode>· {MODE_STYLE[mode].label}</ToolChipMode>
                  </ToolChip>
                )
              })}
            </ToolGrid>

            {isDirty && (
              <div style={{
                display: 'flex', alignItems: 'center', gap: 10, marginTop: 14,
                padding: '10px 14px', borderRadius: 10,
                background: '#fffbeb', border: '1px solid #fde68a',
              }}>
                <span style={{ fontSize: 12, color: '#92400e', flex: 1 }}>
                  ⚠️ Modifications non appliquées — ton ami utilise encore l'ancienne config.
                </span>
                <button
                  onClick={cancelChanges}
                  style={{
                    padding: '6px 14px', borderRadius: 8, border: '1px solid #e5e7eb',
                    background: '#fff', fontSize: 12, fontFamily: 'inherit',
                    cursor: 'pointer', color: '#6b7280',
                  }}
                >
                  Annuler
                </button>
                <button
                  onClick={applyChanges}
                  disabled={permsMut.isPending}
                  style={{
                    padding: '6px 16px', borderRadius: 8, border: 'none',
                    background: '#5b5ef4', color: '#fff', fontSize: 12,
                    fontWeight: 600, fontFamily: 'inherit',
                    cursor: 'pointer', opacity: permsMut.isPending ? 0.6 : 1,
                  }}
                >
                  {permsMut.isPending ? 'Application...' : 'Appliquer'}
                </button>
              </div>
            )}

            <Legend style={{ marginTop: isDirty ? 8 : 10 }}>
              <span>● Désactivé — accès bloqué</span>
              <span style={{ color: '#065f46' }}>● Auto — exécuté immédiatement</span>
              <span style={{ color: '#92400e' }}>● Approbation — tu valides dans le dashboard</span>
            </Legend>
            {myToolIds.length === 0 && (
              <p style={{ fontSize: 12, color: '#9ca3af', marginTop: 8 }}>
                Aucun connecteur configuré — ajoutez-en dans la page Connecteurs.
              </p>
            )}
          </PeerSection>

          {/* ── Section 2 : ce que mon ami m'autorise à utiliser ── */}
          <PeerSection style={{ background: '#fafafa' }}>
            <PeerSectionTitle>
              Ce que {peer.display_name} m'autorise à utiliser sur son Mac
            </PeerSectionTitle>
            {theirTools.length > 0 ? (
              <ToolGrid>
                {theirTools.map(({ id, mode }) => (
                  <ToolChip key={id} $mode={mode} $readonly>
                    <ToolChipName>{labelFor(id)}</ToolChipName>
                    <ToolChipMode>· {MODE_STYLE[mode].label}</ToolChipMode>
                  </ToolChip>
                ))}
              </ToolGrid>
            ) : (
              <p style={{ fontSize: 12, color: '#9ca3af' }}>
                Synchronisation en attente — les permissions s'afficheront quand votre ami sera connecté.
              </p>
            )}
          </PeerSection>
        </>
      )}
    </PeerCard>
  )
}

// ─── Sous-composant : rejoindre via lien ──────────────────────────────────────

function JoinSection() {
  const [link, setLink] = useState('')
  const [name, setName] = useState('')
  const [ok, setOk] = useState(false)
  const qc = useQueryClient()

  const connectMut = useMutation({
    mutationFn: () => api.connectPeer(link, name),
    onSuccess: () => {
      setOk(true)
      setLink('')
      setName('')
      qc.invalidateQueries({ queryKey: ['network-peers'] })
      setTimeout(() => setOk(false), 5000)
    },
  })

  return (
    <Card>
      <CardHeader>
        <CardTitle>Rejoindre via un lien</CardTitle>
      </CardHeader>
      <CardBody>
        <FormRow>
          <Label>Lien d'invitation reçu</Label>
          <Input
            type="text"
            value={link}
            onChange={e => setLink(e.target.value)}
            placeholder="osmozzz://invite/..."
          />
          <Hint>Lien envoyé par votre collègue depuis son dashboard.</Hint>
        </FormRow>
        <FormRow>
          <Label>Nom affiché pour ce peer</Label>
          <Input
            type="text"
            value={name}
            onChange={e => setName(e.target.value)}
            placeholder="Thomas"
          />
        </FormRow>
        {ok && <Alert $variant="success">✓ Connexion initiée — le peer apparaîtra dans la liste sous peu.</Alert>}
        {connectMut.isError && (
          <Alert $variant="error">Erreur : {String(connectMut.error)}</Alert>
        )}
        <PrimaryBtn
          onClick={() => connectMut.mutate()}
          disabled={!link || !name || connectMut.isPending}
        >
          <icons.UserPlus size={14} />
          {connectMut.isPending ? 'Connexion...' : 'Se connecter'}
        </PrimaryBtn>
      </CardBody>
    </Card>
  )
}


// ─── Flux P2P : même layout que "Flux d'actions" (ActionsPage) ───────────────

function P2pFluxSection({ peerFilter }: { peerFilter?: { id: string; name: string } }) {
  const qc = useQueryClient()

  const { data: pending = [], isLoading: loadingPending } = useQuery({
    queryKey: ['p2p-pending'],
    queryFn: api.getP2pPending,
    refetchInterval: 3000,
  })

  const { data: history = [], isLoading: loadingHistory } = useQuery({
    queryKey: ['network-history'],
    queryFn: api.getNetworkHistory,
    refetchInterval: 5000,
  })

  function invalidate() {
    qc.invalidateQueries({ queryKey: ['p2p-pending'] })
  }

  const nowTs = Math.floor(Date.now() / 1000)
  const visiblePending = (pending as ActionRequest[])
    .filter(a => a.expires_at > nowTs)
    .filter(a => !peerFilter || a.preview.includes(peerFilter.name))
  const historyDone = (pending as ActionRequest[])
    .filter(a => a.status !== 'pending')
    .filter(a => !peerFilter || a.preview.includes(peerFilter.name))

  // Convertit QueryHistoryEntry → format attendu par JournalEntryRow
  // tool = "peer_name:query" (pour tool_call) ou "peer_name:search" (pour search)
  const toJournalEntry = (e: QueryHistoryEntry) => ({
    ts: e.ts,
    tool: e.kind === 'tool_call' ? `${e.peer_name}:${e.query}` : `${e.peer_name}:search`,
    query: e.kind === 'search' ? `"${e.query}"` : '',
    results: e.results_count,
    blocked: e.blocked,
    data: undefined,
  })

  return (
    <Card>
      <CardHeader>
        <CardTitle>Flux d'actions P2P</CardTitle>
        {visiblePending.length > 0 && (
          <span style={{ fontSize: 10, fontWeight: 700, minWidth: 18, height: 18, padding: '0 5px', background: '#ef4444', color: '#fff', borderRadius: 99, display: 'inline-flex', alignItems: 'center', justifyContent: 'center' }}>
            {visiblePending.length}
          </span>
        )}
      </CardHeader>
      <div style={{ display: 'flex', gap: 0, alignItems: 'flex-start', borderTop: '1px solid #e8eaed' }}>

        {/* ── EN ATTENTE ── */}
        <div style={{ flex: 1, minWidth: 0, padding: '16px 20px', borderRight: '1px solid #e8eaed' }}>
          <div style={{ fontSize: 12, fontWeight: 600, color: '#6b7280', textTransform: 'uppercase', letterSpacing: '.06em', marginBottom: 10 }}>
            En attente
          </div>
          {loadingPending && <div style={{ color: '#9ca3af', fontSize: 12, padding: '20px 0' }}>Chargement...</div>}
          {!loadingPending && visiblePending.length === 0 && (
            <EmptyMsg>Aucune demande en attente.<br />Quand un collègue utilise un outil en mode <strong>Approbation</strong>, sa demande apparaît ici.</EmptyMsg>
          )}
          <CardList>
            {visiblePending.map(a => (
              <ActionCardItem
                key={a.id}
                action={a}
                onDecision={invalidate}
                onApprove={api.approveP2pAction}
                onReject={api.rejectP2pAction}
              />
            ))}
          </CardList>
        </div>

        {/* ── HISTORIQUE ── */}
        <div style={{ flex: 1, minWidth: 0, padding: '16px 20px', borderRight: '1px solid #e8eaed' }}>
          <div style={{ fontSize: 12, fontWeight: 600, color: '#6b7280', textTransform: 'uppercase', letterSpacing: '.06em', marginBottom: 10 }}>
            Historique
          </div>
          {!loadingPending && historyDone.length === 0 && <EmptyMsg>Aucune action dans l'historique.</EmptyMsg>}
          <CardList>
            {historyDone.map(a => (
              <ActionCardItem key={a.id} action={a} onDecision={invalidate} />
            ))}
          </CardList>
        </div>

        {/* ── JOURNAL D'ACCÈS ── */}
        <div style={{ flex: 1, minWidth: 0, padding: '16px 20px' }}>
          <div style={{ fontSize: 12, fontWeight: 600, color: '#6b7280', textTransform: 'uppercase', letterSpacing: '.06em', marginBottom: 10 }}>
            Journal d'accès
          </div>
          {loadingHistory && <div style={{ color: '#9ca3af', fontSize: 12, padding: '20px 0' }}>Chargement...</div>}
          {!loadingHistory && history.length === 0 && (
            <EmptyMsg>Aucune activité enregistrée.<br />Le journal se remplit dès qu'un collègue accède à tes données.</EmptyMsg>
          )}
          <JournalList>
            {history
              .filter((e: QueryHistoryEntry) => !peerFilter || e.peer_id === peerFilter.id || e.peer_name === peerFilter.name)
              .slice(0, 50)
              .map((e: QueryHistoryEntry, i: number) => (
                <JournalEntryRow key={i} entry={toJournalEntry(e)} />
              ))}
          </JournalList>
        </div>

      </div>
    </Card>
  )
}

// ─── Tab bar (même pattern qu'ActionsPage) ────────────────────────────────────

const TopTabBar = styled.div`
  display: flex; align-items: center; gap: 2px;
  border-bottom: 1px solid #e8eaed;
  margin-bottom: 24px; padding-bottom: 0;
`

const TopTabItem = styled.button<{ $active: boolean }>`
  display: flex; align-items: center; gap: 7px;
  padding: 8px 14px 10px; border: none; cursor: pointer; background: transparent;
  font-size: 13px; font-weight: ${({ $active }) => $active ? '600' : '500'};
  color: ${({ $active }) => $active ? '#1a1d23' : '#6b7280'};
  border-bottom: 2px solid ${({ $active }) => $active ? '#5b5ef4' : 'transparent'};
  margin-bottom: -1px;
  transition: color .15s, border-color .15s;
  font-family: inherit;
  &:hover { color: #1a1d23; }
`

const TabBadge = styled.span`
  display: inline-flex; align-items: center; justify-content: center;
  min-width: 18px; height: 18px; padding: 0 5px;
  background: #ef4444; color: #fff; border-radius: 99px;
  font-size: 10px; font-weight: 700;
`

export default function NetworkPage() {
  const [tab, setTab] = useState<'reseau' | 'flux'>('reseau')
  const [selectedPeerId, setSelectedPeerId] = useState<string | null>(null)
  const qc = useQueryClient()

  const { data: peersReal = [], isLoading } = useQuery({
    queryKey: ['network-peers'],
    queryFn: api.getNetworkPeers,
    refetchInterval: 5000,
  })

  const { data: p2pPending = [] } = useQuery({
    queryKey: ['p2p-pending'],
    queryFn: api.getP2pPending,
    refetchInterval: 5000,
  })

  // SSE — mise à jour instantanée du statut connecté/déconnecté
  useEffect(() => {
    const es = new EventSource('/api/network/stream')
    const refresh = () => qc.invalidateQueries({ queryKey: ['network-peers'] })
    es.addEventListener('peer_connected', refresh)
    es.addEventListener('peer_disconnected', refresh)
    return () => es.close()
  }, [qc])

  const nowTs = Math.floor(Date.now() / 1000)
  const pendingCount = (p2pPending as ActionRequest[]).filter(a => a.expires_at > nowTs).length

  // Peer sélectionné : garder la sélection ou défaut au premier peer
  const effectiveId = selectedPeerId ?? peersReal[0]?.peer_id ?? null
  const selectedPeer = peersReal.find(p => p.peer_id === effectiveId) ?? null

  return (
    <Page>
      <PageHeader>
        <PageTitle>Réseau</PageTitle>
      </PageHeader>

      {/* ── Tab bar ── */}
      <TopTabBar>
        <TopTabItem $active={tab === 'reseau'} onClick={() => setTab('reseau')}>
          <icons.Network size={14} />
          Réseau
        </TopTabItem>
        <TopTabItem $active={tab === 'flux'} onClick={() => setTab('flux')}>
          <icons.Zap size={14} />
          Flux d'actions
          {pendingCount > 0 && <TabBadge>{pendingCount}</TabBadge>}
        </TopTabItem>
      </TopTabBar>

      {/* ── Onglet Réseau : identité + rejoindre seulement ── */}
      {tab === 'reseau' && (
        <>
          <SectionTitle>Mon identité</SectionTitle>
          <IdentitySection />
          <SectionTitle>Rejoindre un collègue</SectionTitle>
          <JoinSection />
        </>
      )}

      {/* ── Onglet Flux d'actions : peers en haut, flux filtré en bas ── */}
      {tab === 'flux' && (
        <>
          <SectionTitle>Mes collègues</SectionTitle>
          {isLoading ? (
            <Card>
              <CardBody><p style={{ color: '#9ca3af', fontSize: 13 }}>Chargement...</p></CardBody>
            </Card>
          ) : peersReal.length === 0 ? (
            <Card>
              <CardBody>
                <p style={{ color: '#9ca3af', fontSize: 13 }}>
                  Aucun collègue connecté — partagez votre lien d'invitation depuis l'onglet Réseau.
                </p>
              </CardBody>
            </Card>
          ) : (
            peersReal.map(peer => (
              <PeerCardItem
                key={peer.peer_id}
                peer={peer}
                selected={effectiveId === peer.peer_id}
                onSelect={() => setSelectedPeerId(peer.peer_id)}
              />
            ))
          )}

          {selectedPeer && (
            <>
              <SectionTitle>Flux de {selectedPeer.display_name}</SectionTitle>
              <P2pFluxSection
                peerFilter={{ id: selectedPeer.peer_id, name: selectedPeer.display_name }}
              />
            </>
          )}
        </>
      )}
    </Page>
  )
}
