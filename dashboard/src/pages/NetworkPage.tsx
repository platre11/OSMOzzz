import { useState } from 'react'
import styled from 'styled-components'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { api } from '../api'
import type { PeerResponse, PeerPermissions, QueryHistoryEntry } from '../api'
import { ALL_SOURCES } from '../api'
import { icons } from '../lib/assets'

// ─── Styles ──────────────────────────────────────────────────────────────────

const Page = styled.div`
  display: flex;
  flex-direction: column;
  gap: 24px;
`

const PageTitle = styled.h1`
  font-size: 22px;
  font-weight: 700;
  color: #1a1d23;
  letter-spacing: -.02em;
`

const SectionTitle = styled.h2`
  font-size: 13px;
  font-weight: 600;
  color: #6b7280;
  text-transform: uppercase;
  letter-spacing: .06em;
  margin-bottom: -8px;
`

const Card = styled.div`
  background: #fff;
  border: 1px solid #e8eaed;
  border-radius: 14px;
  overflow: hidden;
  box-shadow: 0 1px 3px rgba(0,0,0,.05);
`

const CardHeader = styled.div`
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 20px 24px 16px;
  border-bottom: 1px solid #f3f4f6;
`

const CardBody = styled.div`
  padding: 20px 24px 24px;
  display: flex;
  flex-direction: column;
  gap: 14px;
`

const PeerCard = styled.div`
  background: #fff;
  border: 1px solid #e8eaed;
  border-radius: 14px;
  overflow: hidden;
`

const PeerHeader = styled.div`
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px;
  border-bottom: 1px solid #f9fafb;
`

const PeerInfo = styled.div`
  display: flex;
  align-items: center;
  gap: 12px;
`

const PeerAvatar = styled.div<{ $connected: boolean }>`
  width: 38px;
  height: 38px;
  border-radius: 50%;
  background: ${({ $connected }) => $connected ? '#ede9fe' : '#f3f4f6'};
  color: ${({ $connected }) => $connected ? '#5b21b6' : '#9ca3af'};
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: 14px;
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

const PeerActions = styled.div`
  display: flex;
  align-items: center;
  gap: 8px;
`

const IconBtn = styled.button<{ $danger?: boolean }>`
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 8px;
  border: 1px solid #e5e7eb;
  background: #fff;
  color: ${({ $danger }) => $danger ? '#ef4444' : '#6b7280'};
  cursor: pointer;
  transition: all .15s;

  &:hover {
    background: ${({ $danger }) => $danger ? '#fee2e2' : '#f3f4f6'};
    border-color: ${({ $danger }) => $danger ? '#fca5a5' : '#d1d5db'};
  }
`

const SourceTags = styled.div`
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  padding: 12px 20px;
`

const SourceTag = styled.span<{ $allowed: boolean }>`
  font-size: 11px;
  font-weight: 500;
  padding: 3px 8px;
  border-radius: 6px;
  background: ${({ $allowed }) => $allowed ? '#d1fae5' : '#f3f4f6'};
  color: ${({ $allowed }) => $allowed ? '#065f46' : '#9ca3af'};
  cursor: pointer;
  transition: all .15s;
  user-select: none;

  &:hover {
    opacity: .8;
  }
`

const EmptyState = styled.div`
  text-align: center;
  padding: 48px 24px;
  color: #9ca3af;
`

const EmptyTitle = styled.p`
  font-size: 15px;
  font-weight: 600;
  color: #6b7280;
  margin-bottom: 8px;
`

const EmptyDesc = styled.p`
  font-size: 13px;
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

const Label = styled.label`
  font-size: 12px;
  font-weight: 500;
  color: #374151;
  display: block;
  margin-bottom: 6px;
`

const Hint = styled.p`
  font-size: 11px;
  color: #9ca3af;
  margin-top: 4px;
`

const Button = styled.button`
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

const InviteBox = styled.div`
  background: #f8fafc;
  border: 1px solid #e2e8f0;
  border-radius: 10px;
  padding: 14px 16px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
`

const InviteLink = styled.code`
  font-size: 11px;
  color: #475569;
  word-break: break-all;
  flex: 1;
`

const HistoryRow = styled.div`
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 0;
  border-bottom: 1px solid #f9fafb;

  &:last-child { border-bottom: none; }
`

const HistoryQuery = styled.p`
  font-size: 13px;
  color: #1a1d23;
  font-style: italic;
`

const HistoryMeta = styled.p`
  font-size: 11px;
  color: #9ca3af;
  margin-top: 2px;
`

const HistoryRight = styled.div`
  text-align: right;
  flex-shrink: 0;
`

const ResultCount = styled.span<{ $n: number }>`
  font-size: 11px;
  font-weight: 600;
  color: ${({ $n }) => $n > 0 ? '#059669' : '#9ca3af'};
`

// ─── Sous-composant : permissions d'un peer ───────────────────────────────────

function PeerPermissionsPanel({ peer }: { peer: PeerResponse }) {
  const qc = useQueryClient()

  const { data: perms } = useQuery({
    queryKey: ['peer-permissions', peer.peer_id],
    queryFn: () => api.getPeerPermissions(peer.peer_id),
    initialData: {
      allowed_sources: peer.shared_sources,
      max_results_per_query: 10,
    },
  })

  const mutation = useMutation({
    mutationFn: (p: PeerPermissions) => api.setPeerPermissions(peer.peer_id, p),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['peer-permissions', peer.peer_id] }),
  })

  const toggle = (source: string) => {
    if (!perms) return
    const current = perms.allowed_sources
    const next = current.includes(source)
      ? current.filter(s => s !== source)
      : [...current, source]
    mutation.mutate({ ...perms, allowed_sources: next })
  }

  return (
    <SourceTags>
      {ALL_SOURCES.map(src => (
        <SourceTag
          key={src}
          $allowed={perms?.allowed_sources.includes(src) ?? false}
          onClick={() => toggle(src)}
          title={perms?.allowed_sources.includes(src) ? 'Cliquer pour bloquer' : 'Cliquer pour autoriser'}
        >
          {src}
        </SourceTag>
      ))}
    </SourceTags>
  )
}

// ─── Sous-composant : card peer ───────────────────────────────────────────────

function PeerCardItem({ peer }: { peer: PeerResponse }) {
  const [expanded, setExpanded] = useState(false)
  const qc = useQueryClient()

  const deleteMut = useMutation({
    mutationFn: () => api.deletePeer(peer.peer_id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['network-peers'] }),
  })

  const initials = peer.display_name.slice(0, 2).toUpperCase()
  const lastSeen = peer.last_seen
    ? new Date(peer.last_seen * 1000).toLocaleDateString('fr-FR', { day: '2-digit', month: '2-digit', hour: '2-digit', minute: '2-digit' })
    : null

  return (
    <PeerCard>
      <PeerHeader>
        <PeerInfo>
          <PeerAvatar $connected={peer.connected}>{initials}</PeerAvatar>
          <div>
            <PeerName>{peer.display_name}</PeerName>
            <PeerMeta>
              {peer.addresses[0] ?? 'adresse inconnue'}
              {lastSeen && ` · vu le ${lastSeen}`}
            </PeerMeta>
          </div>
        </PeerInfo>
        <PeerActions>
          <StatusDot $on={peer.connected}>
            {peer.connected ? 'Connecté' : 'Hors ligne'}
          </StatusDot>
          <IconBtn
            onClick={() => setExpanded(e => !e)}
            title="Gérer les permissions"
          >
            <icons.Shield size={14} />
          </IconBtn>
          <IconBtn
            $danger
            onClick={() => deleteMut.mutate()}
            disabled={deleteMut.isPending}
            title="Déconnecter et supprimer"
          >
            <icons.UserX size={14} />
          </IconBtn>
        </PeerActions>
      </PeerHeader>
      {expanded && <PeerPermissionsPanel peer={peer} />}
    </PeerCard>
  )
}

// ─── Sous-composant : invitation ──────────────────────────────────────────────

function InviteSection() {
  const [ip, setIp] = useState('')
  const [link, setLink] = useState('')
  const [copied, setCopied] = useState(false)

  const generateMut = useMutation({
    mutationFn: () => api.generateInvite(),
    onSuccess: (data) => setLink(data.link),
  })

  const copy = () => {
    navigator.clipboard.writeText(link)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  return (
    <Card>
      <CardHeader>
        <span style={{ fontSize: 13, fontWeight: 600, color: '#1a1d23' }}>
          Inviter un collègue
        </span>
      </CardHeader>
      <CardBody>
        <div>
          <Label>Votre IP publique ou nom de domaine</Label>
          <Input
            type="text"
            value={ip}
            onChange={e => setIp(e.target.value)}
            placeholder="192.168.1.10 ou mon-bureau.monentreprise.com"
          />
          <Hint>Votre collègue se connectera à cette adresse. Sur un VPN d'entreprise, utilisez l'IP interne.</Hint>
        </div>
        <Button onClick={() => generateMut.mutate()} disabled={!ip || generateMut.isPending}>
          <icons.Link size={14} />
          {generateMut.isPending ? 'Génération...' : 'Générer un lien'}
        </Button>
        {link && (
          <InviteBox>
            <InviteLink>{link}</InviteLink>
            <IconBtn onClick={copy} title="Copier">
              {copied ? <icons.Check size={14} /> : <icons.Copy size={14} />}
            </IconBtn>
          </InviteBox>
        )}
      </CardBody>
    </Card>
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
      setTimeout(() => setOk(false), 4000)
    },
  })

  return (
    <Card>
      <CardHeader>
        <span style={{ fontSize: 13, fontWeight: 600, color: '#1a1d23' }}>
          Rejoindre via un lien
        </span>
      </CardHeader>
      <CardBody>
        <div>
          <Label>Lien d'invitation reçu</Label>
          <Input
            type="text"
            value={link}
            onChange={e => setLink(e.target.value)}
            placeholder="osmozzz://invite/..."
          />
        </div>
        <div>
          <Label>Nom du collègue</Label>
          <Input
            type="text"
            value={name}
            onChange={e => setName(e.target.value)}
            placeholder="Thomas"
          />
        </div>
        {ok && (
          <div style={{ fontSize: 13, background: '#d1fae5', color: '#065f46', borderRadius: 8, padding: '10px 14px' }}>
            ✓ Connexion en cours — le peer apparaîtra dans la liste sous peu.
          </div>
        )}
        {connectMut.isError && (
          <div style={{ fontSize: 13, background: '#fee2e2', color: '#991b1b', borderRadius: 8, padding: '10px 14px' }}>
            Erreur : {String(connectMut.error)}
          </div>
        )}
        <Button onClick={() => connectMut.mutate()} disabled={!link || !name || connectMut.isPending}>
          <icons.UserPlus size={14} />
          {connectMut.isPending ? 'Connexion...' : 'Se connecter'}
        </Button>
      </CardBody>
    </Card>
  )
}

// ─── Sous-composant : historique ──────────────────────────────────────────────

function HistorySection() {
  const { data: history = [] } = useQuery({
    queryKey: ['network-history'],
    queryFn: api.getNetworkHistory,
  })

  const fmt = (ts: number) =>
    new Date(ts * 1000).toLocaleDateString('fr-FR', {
      day: '2-digit', month: '2-digit',
      hour: '2-digit', minute: '2-digit',
    })

  return (
    <Card>
      <CardHeader>
        <span style={{ fontSize: 13, fontWeight: 600, color: '#1a1d23' }}>
          Requêtes reçues
        </span>
        <span style={{ fontSize: 11, color: '#9ca3af' }}>
          {history.length} entrées
        </span>
      </CardHeader>
      <CardBody>
        {history.length === 0 ? (
          <p style={{ fontSize: 13, color: '#9ca3af', textAlign: 'center', padding: '16px 0' }}>
            Aucune requête reçue pour l'instant.
          </p>
        ) : (
          history.map((entry: QueryHistoryEntry, i: number) => (
            <HistoryRow key={i}>
              <div>
                <HistoryQuery>"{entry.query}"</HistoryQuery>
                <HistoryMeta>par {entry.peer_name} · {fmt(entry.ts)}</HistoryMeta>
              </div>
              <HistoryRight>
                <ResultCount $n={entry.results_count}>
                  {entry.blocked ? '🚫 bloqué' : `${entry.results_count} résultat${entry.results_count !== 1 ? 's' : ''}`}
                </ResultCount>
              </HistoryRight>
            </HistoryRow>
          ))
        )}
      </CardBody>
    </Card>
  )
}

// ─── Page principale ──────────────────────────────────────────────────────────

export default function NetworkPage() {
  const { data: peers = [], isLoading } = useQuery({
    queryKey: ['network-peers'],
    queryFn: api.getNetworkPeers,
    refetchInterval: 5000,
  })

  const connectedCount = peers.filter(p => p.connected).length

  return (
    <Page>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <PageTitle>Réseau</PageTitle>
        <span style={{
          fontSize: 12, fontWeight: 600,
          padding: '4px 12px', borderRadius: 20,
          background: connectedCount > 0 ? '#ede9fe' : '#f3f4f6',
          color: connectedCount > 0 ? '#5b21b6' : '#6b7280',
        }}>
          {connectedCount} peer{connectedCount !== 1 ? 's' : ''} connecté{connectedCount !== 1 ? 's' : ''}
        </span>
      </div>

      {/* ── Peers actifs ──────────────────────────────────────────────────── */}
      <SectionTitle>Connexions</SectionTitle>

      {isLoading ? (
        <Card><CardBody><p style={{ color: '#9ca3af', fontSize: 13 }}>Chargement...</p></CardBody></Card>
      ) : peers.length === 0 ? (
        <Card>
          <EmptyState>
            <icons.Network size={32} style={{ color: '#d1d5db', marginBottom: 12 }} />
            <EmptyTitle>Aucun peer connecté</EmptyTitle>
            <EmptyDesc>Invite un collègue ou rejoins un réseau existant ci-dessous.</EmptyDesc>
          </EmptyState>
        </Card>
      ) : (
        peers.map(peer => <PeerCardItem key={peer.peer_id} peer={peer} />)
      )}

      {/* ── Connexion ─────────────────────────────────────────────────────── */}
      <SectionTitle>Connexion</SectionTitle>
      <InviteSection />
      <JoinSection />

      {/* ── Historique ────────────────────────────────────────────────────── */}
      <SectionTitle>Historique des requêtes reçues</SectionTitle>
      <HistorySection />
    </Page>
  )
}
