import { useState } from 'react'
import styled from 'styled-components'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { api } from '../api'
import type { ConfigData, ConnectorStatus } from '../api'
import { icons } from '../lib/assets'

// ─── Styled components ────────────────────────────────────────────────────────

const Page = styled.div`
  display: flex;
  flex-direction: column;
  gap: 32px;
`

const PageHeader = styled.div`
  display: flex;
  flex-direction: column;
  gap: 4px;
`

const PageTitle = styled.h1`
  font-size: 22px;
  font-weight: 700;
  color: #1a1d23;
  letter-spacing: -.02em;
`

const PageSubtitle = styled.p`
  font-size: 13px;
  color: #6b7280;
`

const Section = styled.div`
  display: flex;
  flex-direction: column;
  gap: 12px;
`

const SectionTitle = styled.h2`
  font-size: 11px;
  font-weight: 700;
  color: #9ca3af;
  text-transform: uppercase;
  letter-spacing: .08em;
`

// ─── Active chips ─────────────────────────────────────────────────────────────

const ActiveGrid = styled.div`
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
`

const ActiveChip = styled.button`
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 8px 14px;
  background: #fff;
  border: 1px solid #e8eaed;
  border-radius: 100px;
  font-size: 13px;
  font-weight: 500;
  color: #1a1d23;
  cursor: pointer;
  font-family: inherit;
  transition: border-color .15s, box-shadow .15s;

  &:hover {
    border-color: #c7d2fe;
    box-shadow: 0 2px 8px rgba(91,94,244,.08);
  }
`

const ActiveDot = styled.span`
  width: 7px;
  height: 7px;
  background: #10b981;
  border-radius: 50%;
  flex-shrink: 0;
`

// ─── Available grid ───────────────────────────────────────────────────────────

const AvailableGrid = styled.div`
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 12px;
`

const AvailableCard = styled.button`
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 10px;
  padding: 16px;
  background: #fff;
  border: 1px solid #e8eaed;
  border-radius: 14px;
  cursor: pointer;
  font-family: inherit;
  text-align: left;
  transition: border-color .15s, box-shadow .15s, transform .1s;

  &:hover {
    border-color: #c7d2fe;
    box-shadow: 0 4px 12px rgba(91,94,244,.08);
    transform: translateY(-1px);
  }
`

const AvailableCardName = styled.span`
  font-size: 13px;
  font-weight: 600;
  color: #1a1d23;
`

const AvailableCardDesc = styled.span`
  font-size: 11px;
  color: #9ca3af;
  line-height: 1.4;
`

const AddBadge = styled.span`
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 11px;
  font-weight: 600;
  color: #5b5ef4;
  background: #ede9fe;
  padding: 3px 8px;
  border-radius: 20px;
`

// ─── Modal ────────────────────────────────────────────────────────────────────

const Overlay = styled.div`
  position: fixed;
  inset: 0;
  background: rgba(0,0,0,.35);
  backdrop-filter: blur(2px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  padding: 24px;
`

const Modal = styled.div`
  background: #fff;
  border-radius: 18px;
  box-shadow: 0 20px 60px rgba(0,0,0,.18);
  width: 100%;
  max-width: 480px;
  display: flex;
  flex-direction: column;
  overflow: hidden;
`

const ModalHeader = styled.div`
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 20px 24px;
  border-bottom: 1px solid #f3f4f6;
`

const ModalTitle = styled.h3`
  font-size: 16px;
  font-weight: 700;
  color: #1a1d23;
`

const ModalStatus = styled.span<{ $ok?: boolean }>`
  font-size: 11px;
  font-weight: 600;
  padding: 3px 10px;
  border-radius: 20px;
  background: ${({ $ok }) => $ok ? '#d1fae5' : '#f3f4f6'};
  color: ${({ $ok }) => $ok ? '#065f46' : '#6b7280'};
`

const CloseButton = styled.button`
  display: flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  background: #f3f4f6;
  border: none;
  border-radius: 50%;
  cursor: pointer;
  color: #6b7280;
  transition: background .15s;
  &:hover { background: #e5e7eb; color: #1a1d23; }
`

const ModalBody = styled.div`
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 24px;
`

const FieldGroup = styled.div`
  display: flex;
  flex-direction: column;
  gap: 6px;
`

const FieldRow = styled.div`
  display: flex;
  align-items: center;
  justify-content: space-between;
`

const FieldLabel = styled.label`
  font-size: 12px;
  font-weight: 600;
  color: #374151;
`

const FieldHint = styled.span`
  font-size: 11px;
  color: #9ca3af;
`

const ExternalLink = styled.a`
  font-size: 12px;
  color: #5b5ef4;
  text-decoration: none;
  &:hover { text-decoration: underline; }
`

const Input = styled.input`
  width: 100%;
  font-size: 14px;
  font-family: inherit;
  padding: 10px 14px;
  background: #f9fafb;
  border: 1px solid #e5e7eb;
  border-radius: 10px;
  color: #1a1d23;
  outline: none;
  box-sizing: border-box;
  transition: border-color .15s, box-shadow .15s, background .15s;

  &:focus {
    background: #fff;
    border-color: #5b5ef4;
    box-shadow: 0 0 0 3px rgba(91,94,244,.12);
  }

  &::placeholder { color: #9ca3af; }
`

const SaveButton = styled.button`
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 11px 20px;
  background: #5b5ef4;
  color: #fff;
  border: none;
  border-radius: 10px;
  font-size: 13px;
  font-weight: 600;
  font-family: inherit;
  cursor: pointer;
  transition: background .15s;
  width: 100%;

  &:hover { background: #4a4de3; }
  &:disabled { opacity: .4; cursor: not-allowed; }
`

const SuccessBanner = styled.div`
  font-size: 13px;
  background: #d1fae5;
  color: #065f46;
  border-radius: 8px;
  padding: 10px 14px;
  display: flex;
  align-items: center;
  gap: 8px;
`

const ErrorBanner = styled.div`
  font-size: 13px;
  background: #fee2e2;
  color: #991b1b;
  border-radius: 8px;
  padding: 10px 14px;
`


// ─── Connector definitions ────────────────────────────────────────────────────

type ConnectorId =
  | 'gmail' | 'notion' | 'github' | 'linear' | 'jira'
  | 'supabase' | 'cloudflare' | 'sentry' | 'gitlab'

interface ConnectorDef {
  id: ConnectorId
  name: string
  desc: string
}

const CONNECTORS: ConnectorDef[] = [
  { id: 'gmail',      name: 'Gmail',      desc: 'Emails IMAP' },
  { id: 'notion',     name: 'Notion',     desc: 'Pages & bases' },
  { id: 'github',     name: 'GitHub',     desc: 'Issues & PRs' },
  { id: 'linear',     name: 'Linear',     desc: 'Issues & cycles' },
  { id: 'jira',       name: 'Jira',       desc: 'Tickets & projets' },
  { id: 'gitlab',     name: 'GitLab',     desc: 'Issues & MR' },
  { id: 'supabase',   name: 'Supabase',   desc: 'SQL & Edge Functions' },
  { id: 'sentry',     name: 'Sentry',     desc: 'Erreurs & alertes' },
  { id: 'cloudflare', name: 'Cloudflare', desc: 'Workers & DNS' },
]

// ─── Modal forms ──────────────────────────────────────────────────────────────

interface ModalFormProps {
  id: ConnectorId
  status: ConnectorStatus | undefined
  onClose: () => void
  onSaved: () => void
}

function GmailForm({ status, onClose, onSaved }: ModalFormProps) {
  const qc = useQueryClient()
  const [user, setUser] = useState('')
  const [pass, setPass] = useState('')
  const [ok, setOk] = useState(false)
  const mut = useMutation({
    mutationFn: () => api.saveGmail(user, pass),
    onSuccess: () => {
      setOk(true); setPass('')
      qc.invalidateQueries({ queryKey: ['config'] })
      setTimeout(() => { setOk(false); onSaved() }, 2500)
    },
  })
  return (
    <>
      <ModalHeader>
        <ModalTitle>Gmail IMAP</ModalTitle>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
          <ModalStatus $ok={status?.configured}>
            {status?.configured ? `✓ ${status.display ?? 'Configuré'}` : 'Non configuré'}
          </ModalStatus>
          <CloseButton onClick={onClose}><icons.X size={14} /></CloseButton>
        </div>
      </ModalHeader>
      <ModalBody>
        <FieldGroup>
          <FieldLabel>Adresse Gmail</FieldLabel>
          <Input type="email" value={user} onChange={e => setUser(e.target.value)} placeholder="ton@gmail.com" />
        </FieldGroup>
        <FieldGroup>
          <FieldRow>
            <FieldLabel>Mot de passe d'application</FieldLabel>
            <ExternalLink href="https://myaccount.google.com/apppasswords" target="_blank" rel="noreferrer">Générer ↗</ExternalLink>
          </FieldRow>
          <Input type="password" value={pass} onChange={e => setPass(e.target.value)} placeholder="xxxx xxxx xxxx xxxx" />
        </FieldGroup>
        {ok && <SuccessBanner><icons.CheckCircle2 size={15} /> Gmail configuré ! Redémarre le daemon.</SuccessBanner>}
        {mut.isError && <ErrorBanner>Erreur : {String(mut.error)}</ErrorBanner>}
        <SaveButton onClick={() => mut.mutate()} disabled={!user || !pass || mut.isPending}>
          <icons.Save size={14} />{mut.isPending ? 'Sauvegarde...' : 'Sauvegarder'}
        </SaveButton>
      </ModalBody>
    </>
  )
}

function NotionForm({ status, onClose, onSaved }: ModalFormProps) {
  const qc = useQueryClient()
  const [token, setToken] = useState('')
  const [ok, setOk] = useState(false)
  const mut = useMutation({
    mutationFn: () => api.saveNotion(token),
    onSuccess: () => {
      setOk(true); setToken('')
      qc.invalidateQueries({ queryKey: ['config'] })
      setTimeout(() => { setOk(false); onSaved() }, 2500)
    },
  })
  return (
    <>
      <ModalHeader>
        <ModalTitle>Notion</ModalTitle>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
          <ModalStatus $ok={status?.configured}>{status?.configured ? '✓ Configuré' : 'Non configuré'}</ModalStatus>
          <CloseButton onClick={onClose}><icons.X size={14} /></CloseButton>
        </div>
      </ModalHeader>
      <ModalBody>
        <FieldGroup>
          <FieldRow>
            <FieldLabel>Integration Token</FieldLabel>
            <ExternalLink href="https://www.notion.so/profile/integrations/internal" target="_blank" rel="noreferrer">Créer ↗</ExternalLink>
          </FieldRow>
          <Input type="password" value={token} onChange={e => setToken(e.target.value)} placeholder="ntn_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx" />
        </FieldGroup>
        {ok && <SuccessBanner><icons.CheckCircle2 size={15} /> Notion configuré ! Pages indexées dans l'heure.</SuccessBanner>}
        {mut.isError && <ErrorBanner>Erreur : {String(mut.error)}</ErrorBanner>}
        <SaveButton onClick={() => mut.mutate()} disabled={!token || mut.isPending}>
          <icons.Save size={14} />{mut.isPending ? 'Sauvegarde...' : 'Sauvegarder'}
        </SaveButton>
      </ModalBody>
    </>
  )
}

function GithubForm({ status, onClose, onSaved }: ModalFormProps) {
  const qc = useQueryClient()
  const [token, setToken] = useState('')
  const [ok, setOk] = useState(false)
  const mut = useMutation({
    mutationFn: () => api.saveGithub(token, ''),
    onSuccess: () => {
      setOk(true); setToken('')
      qc.invalidateQueries({ queryKey: ['config'] })
      setTimeout(() => { setOk(false); onSaved() }, 2500)
    },
  })
  return (
    <>
      <ModalHeader>
        <ModalTitle>GitHub</ModalTitle>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
          <ModalStatus $ok={status?.configured}>{status?.configured ? '✓ Configuré' : 'Non configuré'}</ModalStatus>
          <CloseButton onClick={onClose}><icons.X size={14} /></CloseButton>
        </div>
      </ModalHeader>
      <ModalBody>
        <FieldGroup>
          <FieldRow>
            <FieldLabel>Personal Access Token</FieldLabel>
            <ExternalLink href="https://github.com/settings/tokens" target="_blank" rel="noreferrer">Générer ↗</ExternalLink>
          </FieldRow>
          <Input type="password" value={token} onChange={e => setToken(e.target.value)} placeholder="ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx" />
        </FieldGroup>
        {ok && <SuccessBanner><icons.CheckCircle2 size={15} /> GitHub configuré ! Redémarre ton client MCP.</SuccessBanner>}
        {mut.isError && <ErrorBanner>Erreur : {String(mut.error)}</ErrorBanner>}
        <SaveButton onClick={() => mut.mutate()} disabled={!token || mut.isPending}>
          <icons.Save size={14} />{mut.isPending ? 'Sauvegarde...' : 'Sauvegarder'}
        </SaveButton>
      </ModalBody>
    </>
  )
}

function LinearForm({ status, onClose, onSaved }: ModalFormProps) {
  const qc = useQueryClient()
  const [key, setKey] = useState('')
  const [ok, setOk] = useState(false)
  const mut = useMutation({
    mutationFn: () => api.saveLinear(key),
    onSuccess: () => {
      setOk(true); setKey('')
      qc.invalidateQueries({ queryKey: ['config'] })
      setTimeout(() => { setOk(false); onSaved() }, 2500)
    },
  })
  return (
    <>
      <ModalHeader>
        <ModalTitle>Linear</ModalTitle>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
          <ModalStatus $ok={status?.configured}>{status?.configured ? '✓ Configuré' : 'Non configuré'}</ModalStatus>
          <CloseButton onClick={onClose}><icons.X size={14} /></CloseButton>
        </div>
      </ModalHeader>
      <ModalBody>
        <FieldGroup>
          <FieldRow>
            <FieldLabel>Personal API Key</FieldLabel>
            <ExternalLink href="https://linear.app/settings/api" target="_blank" rel="noreferrer">Créer ↗</ExternalLink>
          </FieldRow>
          <Input type="password" value={key} onChange={e => setKey(e.target.value)} placeholder="lin_api_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx" />
        </FieldGroup>
        {ok && <SuccessBanner><icons.CheckCircle2 size={15} /> Linear configuré ! Issues indexées dans l'heure.</SuccessBanner>}
        {mut.isError && <ErrorBanner>Erreur : {String(mut.error)}</ErrorBanner>}
        <SaveButton onClick={() => mut.mutate()} disabled={!key || mut.isPending}>
          <icons.Save size={14} />{mut.isPending ? 'Sauvegarde...' : 'Sauvegarder'}
        </SaveButton>
      </ModalBody>
    </>
  )
}

function JiraForm({ status, onClose, onSaved }: ModalFormProps) {
  const qc = useQueryClient()
  const [url, setUrl] = useState('')
  const [email, setEmail] = useState('')
  const [token, setToken] = useState('')
  const [ok, setOk] = useState(false)
  const mut = useMutation({
    mutationFn: () => api.saveJira(url, email, token),
    onSuccess: () => {
      setOk(true); setToken('')
      qc.invalidateQueries({ queryKey: ['config'] })
      setTimeout(() => { setOk(false); onSaved() }, 2500)
    },
  })
  return (
    <>
      <ModalHeader>
        <ModalTitle>Jira</ModalTitle>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
          <ModalStatus $ok={status?.configured}>{status?.configured ? `✓ ${status.display ?? 'Configuré'}` : 'Non configuré'}</ModalStatus>
          <CloseButton onClick={onClose}><icons.X size={14} /></CloseButton>
        </div>
      </ModalHeader>
      <ModalBody>
        <FieldGroup>
          <FieldRow>
            <FieldLabel>URL de votre instance</FieldLabel>
            <FieldHint>ex: votre-domaine.atlassian.net</FieldHint>
          </FieldRow>
          <Input type="url" value={url} onChange={e => setUrl(e.target.value)} placeholder="https://votre-domaine.atlassian.net" />
        </FieldGroup>
        <FieldGroup>
          <FieldLabel>Email du compte</FieldLabel>
          <Input type="email" value={email} onChange={e => setEmail(e.target.value)} placeholder="votre@email.com" />
        </FieldGroup>
        <FieldGroup>
          <FieldRow>
            <FieldLabel>API Token</FieldLabel>
            <ExternalLink href="https://id.atlassian.com/manage-profile/security/api-tokens" target="_blank" rel="noreferrer">Générer ↗</ExternalLink>
          </FieldRow>
          <Input type="password" value={token} onChange={e => setToken(e.target.value)} placeholder="ATATT3xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx" />
        </FieldGroup>
        {ok && <SuccessBanner><icons.CheckCircle2 size={15} /> Jira configuré ! Redémarre le daemon.</SuccessBanner>}
        {mut.isError && <ErrorBanner>Erreur : {String(mut.error)}</ErrorBanner>}
        <SaveButton onClick={() => mut.mutate()} disabled={!url || !email || !token || mut.isPending}>
          <icons.Save size={14} />{mut.isPending ? 'Sauvegarde...' : 'Sauvegarder'}
        </SaveButton>
      </ModalBody>
    </>
  )
}

function GitlabForm({ status, onClose, onSaved }: ModalFormProps) {
  const qc = useQueryClient()
  const [token, setToken] = useState('')
  const [url, setUrl] = useState('')
  const [groups, setGroups] = useState('')
  const [ok, setOk] = useState(false)
  const mut = useMutation({
    mutationFn: () => api.saveGitlab(token, url, groups),
    onSuccess: () => {
      setOk(true); setToken('')
      qc.invalidateQueries({ queryKey: ['config'] })
      setTimeout(() => { setOk(false); onSaved() }, 2500)
    },
  })
  return (
    <>
      <ModalHeader>
        <ModalTitle>GitLab</ModalTitle>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
          <ModalStatus $ok={status?.configured}>{status?.configured ? '✓ Configuré' : 'Non configuré'}</ModalStatus>
          <CloseButton onClick={onClose}><icons.X size={14} /></CloseButton>
        </div>
      </ModalHeader>
      <ModalBody>
        <FieldGroup>
          <FieldRow>
            <FieldLabel>Personal Access Token</FieldLabel>
            <ExternalLink href="https://gitlab.com/-/user_settings/personal_access_tokens" target="_blank" rel="noreferrer">Générer ↗</ExternalLink>
          </FieldRow>
          <Input type="password" value={token} onChange={e => setToken(e.target.value)} placeholder="glpat-xxxxxxxxxxxxxxxxxxxx" />
        </FieldGroup>
        <FieldGroup>
          <FieldRow>
            <FieldLabel>URL de l'instance</FieldLabel>
            <FieldHint>optionnel — défaut : gitlab.com</FieldHint>
          </FieldRow>
          <Input type="url" value={url} onChange={e => setUrl(e.target.value)} placeholder="https://gitlab.com" />
        </FieldGroup>
        <FieldGroup>
          <FieldRow>
            <FieldLabel>Groupes à indexer</FieldLabel>
            <FieldHint>optionnel — séparés par des virgules</FieldHint>
          </FieldRow>
          <Input type="text" value={groups} onChange={e => setGroups(e.target.value)} placeholder="mon-groupe, mon-autre-groupe" />
        </FieldGroup>
        {ok && <SuccessBanner><icons.CheckCircle2 size={15} /> GitLab configuré ! Redémarre le daemon.</SuccessBanner>}
        {mut.isError && <ErrorBanner>Erreur : {String(mut.error)}</ErrorBanner>}
        <SaveButton onClick={() => mut.mutate()} disabled={!token || mut.isPending}>
          <icons.Save size={14} />{mut.isPending ? 'Sauvegarde...' : 'Sauvegarder'}
        </SaveButton>
      </ModalBody>
    </>
  )
}

function SupabaseForm({ status, onClose, onSaved }: ModalFormProps) {
  const qc = useQueryClient()
  const [token, setToken] = useState('')
  const [projectId, setProjectId] = useState('')
  const [ok, setOk] = useState(false)
  const mut = useMutation({
    mutationFn: () => api.saveSupabase(token, projectId || undefined),
    onSuccess: () => {
      setOk(true); setToken('')
      qc.invalidateQueries({ queryKey: ['config'] })
      setTimeout(() => { setOk(false); onSaved() }, 2500)
    },
  })
  return (
    <>
      <ModalHeader>
        <ModalTitle>Supabase</ModalTitle>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
          <ModalStatus $ok={status?.configured}>{status?.configured ? `✓ ${status.display ?? 'Configuré'}` : 'Non configuré'}</ModalStatus>
          <CloseButton onClick={onClose}><icons.X size={14} /></CloseButton>
        </div>
      </ModalHeader>
      <ModalBody>
        <FieldGroup>
          <FieldRow>
            <FieldLabel>Access Token</FieldLabel>
            <ExternalLink href="https://supabase.com/dashboard/account/tokens" target="_blank" rel="noreferrer">Générer ↗</ExternalLink>
          </FieldRow>
          <Input type="password" value={token} onChange={e => setToken(e.target.value)} placeholder="sbp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx" />
        </FieldGroup>
        <FieldGroup>
          <FieldRow>
            <FieldLabel>Project ID <span style={{ color: '#9ca3af', fontWeight: 400 }}>(optionnel)</span></FieldLabel>
            <FieldHint>limite les tools à un seul projet</FieldHint>
          </FieldRow>
          <Input type="text" value={projectId} onChange={e => setProjectId(e.target.value)} placeholder="abcdefghijklmnop" />
        </FieldGroup>
        {ok && <SuccessBanner><icons.CheckCircle2 size={15} /> Supabase configuré ! Redémarre ton client MCP.</SuccessBanner>}
        {mut.isError && <ErrorBanner>Erreur : {String(mut.error)}</ErrorBanner>}
        <SaveButton onClick={() => mut.mutate()} disabled={!token || mut.isPending}>
          <icons.Save size={14} />{mut.isPending ? 'Sauvegarde...' : 'Sauvegarder'}
        </SaveButton>
      </ModalBody>
    </>
  )
}

function SentryForm({ status, onClose, onSaved }: ModalFormProps) {
  const qc = useQueryClient()
  const [token, setToken] = useState('')
  const [host, setHost] = useState('')
  const [ok, setOk] = useState(false)
  const mut = useMutation({
    mutationFn: () => api.saveSentry(token, host || undefined),
    onSuccess: () => {
      setOk(true); setToken('')
      qc.invalidateQueries({ queryKey: ['config'] })
      setTimeout(() => { setOk(false); onSaved() }, 2500)
    },
  })
  return (
    <>
      <ModalHeader>
        <ModalTitle>Sentry</ModalTitle>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
          <ModalStatus $ok={status?.configured}>{status?.configured ? '✓ Configuré' : 'Non configuré'}</ModalStatus>
          <CloseButton onClick={onClose}><icons.X size={14} /></CloseButton>
        </div>
      </ModalHeader>
      <ModalBody>
        <FieldGroup>
          <FieldRow>
            <FieldLabel>User Auth Token</FieldLabel>
            <ExternalLink href="https://sentry.io/settings/account/api/auth-tokens/" target="_blank" rel="noreferrer">Générer ↗</ExternalLink>
          </FieldRow>
          <Input type="password" value={token} onChange={e => setToken(e.target.value)} placeholder="sntryu_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx" />
        </FieldGroup>
        <FieldGroup>
          <FieldRow>
            <FieldLabel>URL de l'instance</FieldLabel>
            <FieldHint>optionnel — défaut : sentry.io</FieldHint>
          </FieldRow>
          <Input type="url" value={host} onChange={e => setHost(e.target.value)} placeholder="https://sentry.io" />
        </FieldGroup>
        {ok && <SuccessBanner><icons.CheckCircle2 size={15} /> Sentry configuré ! Redémarre le daemon.</SuccessBanner>}
        {mut.isError && <ErrorBanner>Erreur : {String(mut.error)}</ErrorBanner>}
        <SaveButton onClick={() => mut.mutate()} disabled={!token || mut.isPending}>
          <icons.Save size={14} />{mut.isPending ? 'Sauvegarde...' : 'Sauvegarder'}
        </SaveButton>
      </ModalBody>
    </>
  )
}

function CloudflareForm({ status, onClose, onSaved }: ModalFormProps) {
  const qc = useQueryClient()
  const [token, setToken] = useState('')
  const [accountId, setAccountId] = useState('')
  const [ok, setOk] = useState(false)
  const mut = useMutation({
    mutationFn: () => api.saveCloudflare(token, accountId),
    onSuccess: () => {
      setOk(true); setToken('')
      qc.invalidateQueries({ queryKey: ['config'] })
      setTimeout(() => { setOk(false); onSaved() }, 2500)
    },
  })
  return (
    <>
      <ModalHeader>
        <ModalTitle>Cloudflare</ModalTitle>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
          <ModalStatus $ok={status?.configured}>{status?.configured ? `✓ ${status.display ?? 'Configuré'}` : 'Non configuré'}</ModalStatus>
          <CloseButton onClick={onClose}><icons.X size={14} /></CloseButton>
        </div>
      </ModalHeader>
      <ModalBody>
        <FieldGroup>
          <FieldRow>
            <FieldLabel>API Token</FieldLabel>
            <ExternalLink href="https://dash.cloudflare.com/profile/api-tokens" target="_blank" rel="noreferrer">Générer ↗</ExternalLink>
          </FieldRow>
          <Input type="password" value={token} onChange={e => setToken(e.target.value)} placeholder="xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx" />
        </FieldGroup>
        <FieldGroup>
          <FieldRow>
            <FieldLabel>Account ID</FieldLabel>
            <ExternalLink href="https://dash.cloudflare.com/" target="_blank" rel="noreferrer">Trouver ↗</ExternalLink>
          </FieldRow>
          <Input type="text" value={accountId} onChange={e => setAccountId(e.target.value)} placeholder="xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx" />
        </FieldGroup>
        {ok && <SuccessBanner><icons.CheckCircle2 size={15} /> Cloudflare configuré ! Redémarre le daemon.</SuccessBanner>}
        {mut.isError && <ErrorBanner>Erreur : {String(mut.error)}</ErrorBanner>}
        <SaveButton onClick={() => mut.mutate()} disabled={!token || !accountId || mut.isPending}>
          <icons.Save size={14} />{mut.isPending ? 'Sauvegarde...' : 'Sauvegarder'}
        </SaveButton>
      </ModalBody>
    </>
  )
}

const FORM_MAP: Record<ConnectorId, React.ComponentType<ModalFormProps>> = {
  gmail:      GmailForm,
  notion:     NotionForm,
  github:     GithubForm,
  linear:     LinearForm,
  jira:       JiraForm,
  gitlab:     GitlabForm,
  supabase:   SupabaseForm,
  sentry:     SentryForm,
  cloudflare: CloudflareForm,
}

// ─── Page ─────────────────────────────────────────────────────────────────────

export default function ConfigPage() {
  const { data: config } = useQuery<ConfigData>({
    queryKey: ['config'],
    queryFn:  api.getConfig,
  })

  const [open, setOpen] = useState<ConnectorId | null>(null)

  function statusOf(id: ConnectorId): ConnectorStatus | undefined {
    return config?.[id as keyof ConfigData] as ConnectorStatus | undefined
  }

  const active   = CONNECTORS.filter(c => statusOf(c.id)?.configured)
  const available = CONNECTORS.filter(c => !statusOf(c.id)?.configured)

  const ActiveForm = open ? FORM_MAP[open] : null

  return (
    <Page>
      <PageHeader>
        <PageTitle>Connecteurs</PageTitle>
        <PageSubtitle>Connectez vos outils pour les rendre accessibles à votre client IA via MCP.</PageSubtitle>
      </PageHeader>

      {/* ── Connecteurs actifs ─────────────────────────────────────────────── */}
      {active.length > 0 && (
        <Section>
          <SectionTitle>Actifs — {active.length} connecteur{active.length > 1 ? 's' : ''}</SectionTitle>
          <ActiveGrid>
            {active.map(c => (
              <ActiveChip key={c.id} onClick={() => setOpen(c.id)}>
                <ActiveDot />
                {c.name}
                {statusOf(c.id)?.display && (
                  <span style={{ color: '#9ca3af', fontSize: 11 }}>· {statusOf(c.id)?.display}</span>
                )}
              </ActiveChip>
            ))}
          </ActiveGrid>
        </Section>
      )}

      {/* ── Connecteurs disponibles ────────────────────────────────────────── */}
      {available.length > 0 && (
        <Section>
          <SectionTitle>Disponibles</SectionTitle>
          <AvailableGrid>
            {available.map(c => (
              <AvailableCard key={c.id} onClick={() => setOpen(c.id)}>
                <AvailableCardName>{c.name}</AvailableCardName>
                <AvailableCardDesc>{c.desc}</AvailableCardDesc>
                <AddBadge><icons.Plus size={10} /> Connecter</AddBadge>
              </AvailableCard>
            ))}
          </AvailableGrid>
        </Section>
      )}

      {/* ── Modal ─────────────────────────────────────────────────────────── */}
      {open && ActiveForm && (
        <Overlay onClick={e => { if (e.target === e.currentTarget) setOpen(null) }}>
          <Modal onClick={e => e.stopPropagation()}>
            <ActiveForm
              id={open}
              status={statusOf(open)}
              onClose={() => setOpen(null)}
              onSaved={() => setOpen(null)}
            />
          </Modal>
        </Overlay>
      )}
    </Page>
  )
}
