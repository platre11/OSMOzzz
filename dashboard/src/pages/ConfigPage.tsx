import { useState } from 'react'
import styled from 'styled-components'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { api } from '../api'
import type { ConfigData, ConnectorStatus } from '../api'
import { icons } from '../lib/assets'
import { PrivacyPanel } from '../components/PrivacyPanel'

// ─── Styles partagés ──────────────────────────────────────────────────────────

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
  padding: 20px 24px 0;
`


const CardTitle = styled.h3`
  font-size: 13px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: .04em;
  color: #9ca3af;
`

const StatusPill = styled.span<{ $ok?: boolean }>`
  font-size: 11px;
  font-weight: 600;
  padding: 3px 10px;
  border-radius: 20px;
  background: ${({ $ok }) => $ok ? '#d1fae5' : '#f3f4f6'};
  color: ${({ $ok }) => $ok ? '#065f46' : '#6b7280'};
`

const CardBody = styled.div`
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding: 20px 24px 24px;
`

const FieldGroup = styled.div``

const FieldRow = styled.div`
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 6px;
`

const FieldLabel = styled.label`
  font-size: 12px;
  font-weight: 500;
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
  background: #fff;
  border: 1px solid #e5e7eb;
  border-radius: 10px;
  color: #1a1d23;
  outline: none;
  box-sizing: border-box;
  transition: border-color .15s, box-shadow .15s;

  &:focus {
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

const InfoBox = styled.div`
  background: #fffbeb;
  border: 1px solid #fde68a;
  border-radius: 14px;
  padding: 20px 24px;
  display: flex;
  flex-direction: column;
  gap: 8px;
`

const InfoTitle = styled.p`
  font-size: 13px;
  font-weight: 600;
  color: #92400e;
`

const InfoDesc = styled.p`
  font-size: 12px;
  color: #a16207;
  line-height: 1.5;
`

const CodeBlock = styled.code`
  display: block;
  background: #1a1d23;
  color: #a5f3fc;
  font-size: 13px;
  padding: 12px 16px;
  border-radius: 8px;
  font-family: 'SF Mono', Monaco, monospace;
  margin-top: 4px;
`

// ─── Composant générique : connecteur à 1 seul champ token ───────────────────

interface SimpleTokenCardProps {
  title: string
  status: ConnectorStatus | undefined
  tokenLabel?: string
  tokenPlaceholder: string
  linkLabel: string
  linkUrl: string
  onSave: (token: string) => Promise<void>
  successMsg?: string
}

function SimpleTokenCard({
  title, status, tokenLabel = 'Token API', tokenPlaceholder,
  linkLabel, linkUrl, onSave, successMsg,
}: SimpleTokenCardProps) {
  const qc = useQueryClient()
  const [token, setToken] = useState('')
  const [ok, setOk] = useState(false)

  const mutation = useMutation({
    mutationFn: () => onSave(token),
    onSuccess: () => {
      setOk(true)
      setToken('')
      qc.invalidateQueries({ queryKey: ['config'] })
      setTimeout(() => setOk(false), 4000)
    },
  })

  return (
    <Card>
      <CardHeader>
        <CardTitle>{title}</CardTitle>
        <StatusPill $ok={status?.configured}>
          {status?.configured
            ? (status.display ? `✓ ${status.display}` : '✓ Configuré')
            : 'Non configuré'}
        </StatusPill>
      </CardHeader>
      <CardBody>
        <FieldGroup>
          <FieldRow>
            <FieldLabel>{tokenLabel}</FieldLabel>
            <ExternalLink href={linkUrl} target="_blank" rel="noreferrer">
              {linkLabel} ↗
            </ExternalLink>
          </FieldRow>
          <Input
            type="password"
            value={token}
            onChange={e => setToken(e.target.value)}
            placeholder={tokenPlaceholder}
          />
        </FieldGroup>
        {ok && (
          <SuccessBanner>
            <icons.CheckCircle2 size={15} />
            {successMsg ?? `${title} configuré ! Redémarre le daemon pour activer la sync.`}
          </SuccessBanner>
        )}
        {mutation.isError && (
          <ErrorBanner>Erreur : {String(mutation.error)}</ErrorBanner>
        )}
        <SaveButton
          onClick={() => mutation.mutate()}
          disabled={!token || mutation.isPending}
        >
          <icons.Save size={14} />
          {mutation.isPending ? 'Sauvegarde...' : 'Sauvegarder'}
        </SaveButton>
      </CardBody>
    </Card>
  )
}

// ─── Page principale ──────────────────────────────────────────────────────────

export default function ConfigPage() {
  const qc = useQueryClient()

  const { data: config } = useQuery<ConfigData>({
    queryKey: ['config'],
    queryFn:  api.getConfig,
  })

  // ── Gmail ─────────────────────────────────────────────────────────────────
  const [gmailUser, setGmailUser] = useState('')
  const [gmailPass, setGmailPass] = useState('')
  const [gmailOk,   setGmailOk]  = useState(false)
  const gmailMut = useMutation({
    mutationFn: () => api.saveGmail(gmailUser, gmailPass),
    onSuccess: () => {
      setGmailOk(true); setGmailPass('')
      qc.invalidateQueries({ queryKey: ['config'] })
      setTimeout(() => setGmailOk(false), 4000)
    },
  })

  // ── GitHub ────────────────────────────────────────────────────────────────
  const [ghToken, setGhToken] = useState('')
  const [ghRepos] = useState('')
  const [ghOk,    setGhOk]   = useState(false)
  const ghMut = useMutation({
    mutationFn: () => api.saveGithub(ghToken, ghRepos),
    onSuccess: () => {
      setGhOk(true); setGhToken('')
      qc.invalidateQueries({ queryKey: ['config'] })
      setTimeout(() => setGhOk(false), 4000)
    },
  })

  // ── Jira ──────────────────────────────────────────────────────────────────
  const [jiraUrl,   setJiraUrl]   = useState('')
  const [jiraEmail, setJiraEmail] = useState('')
  const [jiraToken, setJiraToken] = useState('')
  const [jiraOk,    setJiraOk]   = useState(false)
  const jiraMut = useMutation({
    mutationFn: () => api.saveJira(jiraUrl, jiraEmail, jiraToken),
    onSuccess: () => {
      setJiraOk(true); setJiraToken('')
      qc.invalidateQueries({ queryKey: ['config'] })
      setTimeout(() => setJiraOk(false), 4000)
    },
  })

  // ── Slack, Trello, GitLab, Airtable, Obsidian, Todoist ───────────────────
  // Cards désactivées dans l'UI — pour réactiver : rajouter les useState/useMutation ici
  // et décommenter le JSX correspondant plus bas dans le return

  return (
    <Page>
      <PageTitle>Configuration</PageTitle>

      {/* ── Confidentialité ───────────────────────────────────────────────── */}
      <SectionTitle>Sécurité & Confidentialité</SectionTitle>
      <PrivacyPanel />

      {/* ── Email ─────────────────────────────────────────────────────────── */}
      <SectionTitle>Email</SectionTitle>

      <Card>
        <CardHeader>
          <CardTitle>Gmail IMAP</CardTitle>
          <StatusPill $ok={config?.gmail?.configured}>
            {config?.gmail?.configured
              ? `✓ ${config.gmail.display ?? 'Configuré'}`
              : 'Non configuré'}
          </StatusPill>
        </CardHeader>
        <CardBody>
          <FieldGroup>
            <FieldLabel>Adresse Gmail</FieldLabel>
            <Input
              type="email"
              value={gmailUser}
              onChange={e => setGmailUser(e.target.value)}
              placeholder="ton@gmail.com"
            />
          </FieldGroup>
          <FieldGroup>
            <FieldRow>
              <FieldLabel>Mot de passe d'application</FieldLabel>
              <ExternalLink href="https://myaccount.google.com/apppasswords" target="_blank" rel="noreferrer">
                Générer sur Google ↗
              </ExternalLink>
            </FieldRow>
            <Input
              type="password"
              value={gmailPass}
              onChange={e => setGmailPass(e.target.value)}
              placeholder="xxxx xxxx xxxx xxxx"
            />
          </FieldGroup>
          {gmailOk && <SuccessBanner><icons.CheckCircle2 size={15} /> Gmail configuré ! Redémarre le daemon.</SuccessBanner>}
          {gmailMut.isError && <ErrorBanner>Erreur : {String(gmailMut.error)}</ErrorBanner>}
          <SaveButton onClick={() => gmailMut.mutate()} disabled={!gmailUser || !gmailPass || gmailMut.isPending}>
            <icons.Save size={14} />
            {gmailMut.isPending ? 'Sauvegarde...' : 'Sauvegarder'}
          </SaveButton>
        </CardBody>
      </Card>

      {/* ── Gestion de projet ─────────────────────────────────────────────── */}
      <SectionTitle>Gestion de projet</SectionTitle>

      <SimpleTokenCard
        title="Notion"
        status={config?.notion}
        tokenLabel="Integration Token"
        tokenPlaceholder="ntn_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
        linkLabel="Créer une intégration"
        linkUrl="https://www.notion.so/profile/integrations/internal"
        onSave={token => api.saveNotion(token)}
        successMsg="Notion configuré ! Vos pages seront indexées dans l'heure."
      />

      <Card>
        <CardHeader>
          <CardTitle>GitHub</CardTitle>
          <StatusPill $ok={config?.github?.configured}>
            {config?.github?.configured ? '✓ Configuré' : 'Non configuré'}
          </StatusPill>
        </CardHeader>
        <CardBody>
          <FieldGroup>
            <FieldRow>
              <FieldLabel>Personal Access Token</FieldLabel>
              <ExternalLink href="https://github.com/settings/tokens" target="_blank" rel="noreferrer">
                Générer ↗
              </ExternalLink>
            </FieldRow>
            <Input
              type="password"
              value={ghToken}
              onChange={e => setGhToken(e.target.value)}
              placeholder="ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
            />
          </FieldGroup>
          {ghOk && <SuccessBanner><icons.CheckCircle2 size={15} /> GitHub configuré ! Redémarre Claude Desktop.</SuccessBanner>}
          {ghMut.isError && <ErrorBanner>Erreur : {String(ghMut.error)}</ErrorBanner>}
          <SaveButton onClick={() => ghMut.mutate()} disabled={!ghToken || ghMut.isPending}>
            <icons.Save size={14} />
            {ghMut.isPending ? 'Sauvegarde...' : 'Sauvegarder'}
          </SaveButton>
        </CardBody>
      </Card>

      <SimpleTokenCard
        title="Linear"
        status={config?.linear}
        tokenLabel="Personal API Key"
        tokenPlaceholder="lin_api_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
        linkLabel="Créer une clé"
        linkUrl="https://linear.app/settings/api"
        onSave={key => api.saveLinear(key)}
        successMsg="Linear configuré ! Vos issues seront indexées dans l'heure."
      />

      <Card>
        <CardHeader>
          <CardTitle>Jira</CardTitle>
          <StatusPill $ok={config?.jira?.configured}>
            {config?.jira?.configured
              ? `✓ ${config.jira.display ?? 'Configuré'}`
              : 'Non configuré'}
          </StatusPill>
        </CardHeader>
        <CardBody>
          <FieldGroup>
            <FieldRow>
              <FieldLabel>URL de votre instance</FieldLabel>
              <FieldHint>ex: votre-domaine.atlassian.net</FieldHint>
            </FieldRow>
            <Input
              type="url"
              value={jiraUrl}
              onChange={e => setJiraUrl(e.target.value)}
              placeholder="https://votre-domaine.atlassian.net"
            />
          </FieldGroup>
          <FieldGroup>
            <FieldLabel>Email du compte</FieldLabel>
            <Input
              type="email"
              value={jiraEmail}
              onChange={e => setJiraEmail(e.target.value)}
              placeholder="votre@email.com"
            />
          </FieldGroup>
          <FieldGroup>
            <FieldRow>
              <FieldLabel>API Token</FieldLabel>
              <ExternalLink href="https://id.atlassian.com/manage-profile/security/api-tokens" target="_blank" rel="noreferrer">
                Générer ↗
              </ExternalLink>
            </FieldRow>
            <Input
              type="password"
              value={jiraToken}
              onChange={e => setJiraToken(e.target.value)}
              placeholder="ATATT3xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
            />
          </FieldGroup>
          {jiraOk && <SuccessBanner><icons.CheckCircle2 size={15} /> Jira configuré ! Redémarre le daemon.</SuccessBanner>}
          {jiraMut.isError && <ErrorBanner>Erreur : {String(jiraMut.error)}</ErrorBanner>}
          <SaveButton onClick={() => jiraMut.mutate()} disabled={!jiraUrl || !jiraEmail || !jiraToken || jiraMut.isPending}>
            <icons.Save size={14} />
            {jiraMut.isPending ? 'Sauvegarde...' : 'Sauvegarder'}
          </SaveButton>
        </CardBody>
      </Card>

      {/* ── Trello — désactivé temporairement (non prioritaire) ──────────────
      <Card>
        <CardHeader>
          <CardTitle>Trello</CardTitle>
          <StatusPill $ok={config?.trello?.configured}>
            {config?.trello?.configured ? '✓ Configuré' : 'Non configuré'}
          </StatusPill>
        </CardHeader>
        <CardBody>
          <FieldGroup>
            <FieldRow>
              <FieldLabel>API Key</FieldLabel>
              <ExternalLink href="https://trello.com/power-ups/admin" target="_blank" rel="noreferrer">
                Obtenir ↗
              </ExternalLink>
            </FieldRow>
            <Input type="password" value={trelloKey} onChange={e => setTrelloKey(e.target.value)} placeholder="xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx" />
          </FieldGroup>
          <FieldGroup>
            <FieldRow>
              <FieldLabel>Token</FieldLabel>
              <FieldHint>généré via l'autorisation de votre clé</FieldHint>
            </FieldRow>
            <Input type="password" value={trelloToken} onChange={e => setTrelloToken(e.target.value)} placeholder="xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx" />
          </FieldGroup>
          {trelloOk && <SuccessBanner><icons.CheckCircle2 size={15} /> Trello configuré ! Redémarre le daemon.</SuccessBanner>}
          {trelloMut.isError && <ErrorBanner>Erreur : {String(trelloMut.error)}</ErrorBanner>}
          <SaveButton onClick={() => trelloMut.mutate()} disabled={!trelloKey || !trelloToken || trelloMut.isPending}>
            <icons.Save size={14} />
            {trelloMut.isPending ? 'Sauvegarde...' : 'Sauvegarder'}
          </SaveButton>
        </CardBody>
      </Card>
      ── fin Trello ─────────────────────────────────────────────────────────── */}

      {/* ── Communication ─────────────────────────────────────────────────── */}
      {/* ── Slack — désactivé temporairement (non prioritaire) ───────────────
      <SectionTitle>Communication</SectionTitle>
      <Card>
        <CardHeader>
          <CardTitle>Slack</CardTitle>
          <StatusPill $ok={config?.slack?.configured}>
            {config?.slack?.configured ? '✓ Configuré' : 'Non configuré'}
          </StatusPill>
        </CardHeader>
        <CardBody>
          <FieldGroup>
            <FieldRow>
              <FieldLabel>Bot Token (xoxb-...)</FieldLabel>
              <ExternalLink href="https://api.slack.com/apps" target="_blank" rel="noreferrer">Créer une app ↗</ExternalLink>
            </FieldRow>
            <Input type="password" value={slackToken} onChange={e => setSlackToken(e.target.value)} placeholder="xoxb-xxxxxxxxxxxx-xxxxxxxxxxxx-xxxxxxxxxxxxxxxxxxxxxxxxxxxx" />
          </FieldGroup>
          <FieldGroup>
            <FieldRow>
              <FieldLabel>Team ID (Workspace ID)</FieldLabel>
              <FieldHint>commence par T — visible dans l'URL Slack</FieldHint>
            </FieldRow>
            <Input type="text" value={slackTeamId} onChange={e => setSlackTeamId(e.target.value)} placeholder="TXXXXXXXXXX" />
          </FieldGroup>
          <FieldGroup>
            <FieldRow>
              <FieldLabel>Channels à indexer</FieldLabel>
              <FieldHint>séparés par des virgules</FieldHint>
            </FieldRow>
            <Input type="text" value={slackChannels} onChange={e => setSlackChannels(e.target.value)} placeholder="general, random, dev" />
          </FieldGroup>
          {slackOk && <SuccessBanner><icons.CheckCircle2 size={15} /> Slack configuré ! Redémarre le daemon.</SuccessBanner>}
          {slackMut.isError && <ErrorBanner>Erreur : {String(slackMut.error)}</ErrorBanner>}
          <SaveButton onClick={() => slackMut.mutate()} disabled={!slackToken || !slackTeamId || slackMut.isPending}>
            <icons.Save size={14} />
            {slackMut.isPending ? 'Sauvegarde...' : 'Sauvegarder'}
          </SaveButton>
        </CardBody>
      </Card>
      ── fin Slack ──────────────────────────────────────────────────────────── */}

      {/* ── Tâches — désactivé temporairement (non prioritaire) ──────────────
      <SectionTitle>Tâches</SectionTitle>
      <SimpleTokenCard
        title="Todoist"
        status={config?.todoist}
        tokenLabel="API Token"
        tokenPlaceholder="xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
        linkLabel="Obtenir le token"
        linkUrl="https://todoist.com/app/settings/integrations/developer"
        onSave={token => api.saveTodoist(token)}
        successMsg="Todoist configuré ! Vos tâches seront indexées toutes les 15 min."
      />
      ── fin Todoist ────────────────────────────────────────────────────────── */}

      {/* ── Développement — désactivé temporairement (non prioritaire) ───────
      <SectionTitle>Développement</SectionTitle>
      <Card>
        <CardHeader>
          <CardTitle>GitLab</CardTitle>
          <StatusPill $ok={config?.gitlab?.configured}>
            {config?.gitlab?.configured ? '✓ Configuré' : 'Non configuré'}
          </StatusPill>
        </CardHeader>
        <CardBody>
          <FieldGroup>
            <FieldRow>
              <FieldLabel>Personal Access Token</FieldLabel>
              <ExternalLink href="https://gitlab.com/-/user_settings/personal_access_tokens" target="_blank" rel="noreferrer">Générer ↗</ExternalLink>
            </FieldRow>
            <Input type="password" value={glToken} onChange={e => setGlToken(e.target.value)} placeholder="glpat-xxxxxxxxxxxxxxxxxxxx" />
          </FieldGroup>
          <FieldGroup>
            <FieldRow>
              <FieldLabel>URL de l'instance</FieldLabel>
              <FieldHint>optionnel — défaut : gitlab.com</FieldHint>
            </FieldRow>
            <Input type="url" value={glUrl} onChange={e => setGlUrl(e.target.value)} placeholder="https://gitlab.com" />
          </FieldGroup>
          <FieldGroup>
            <FieldRow>
              <FieldLabel>Groupes à indexer</FieldLabel>
              <FieldHint>optionnel — séparés par des virgules</FieldHint>
            </FieldRow>
            <Input type="text" value={glGroups} onChange={e => setGlGroups(e.target.value)} placeholder="mon-groupe, mon-autre-groupe" />
          </FieldGroup>
          {glOk && <SuccessBanner><icons.CheckCircle2 size={15} /> GitLab configuré ! Redémarre le daemon.</SuccessBanner>}
          {glMut.isError && <ErrorBanner>Erreur : {String(glMut.error)}</ErrorBanner>}
          <SaveButton onClick={() => glMut.mutate()} disabled={!glToken || glMut.isPending}>
            <icons.Save size={14} />
            {glMut.isPending ? 'Sauvegarde...' : 'Sauvegarder'}
          </SaveButton>
        </CardBody>
      </Card>
      <Card>
        <CardHeader>
          <CardTitle>Airtable</CardTitle>
          <StatusPill $ok={config?.airtable?.configured}>
            {config?.airtable?.configured ? '✓ Configuré' : 'Non configuré'}
          </StatusPill>
        </CardHeader>
        <CardBody>
          <FieldGroup>
            <FieldRow>
              <FieldLabel>Personal Access Token</FieldLabel>
              <ExternalLink href="https://airtable.com/create/tokens" target="_blank" rel="noreferrer">Créer un token ↗</ExternalLink>
            </FieldRow>
            <Input type="password" value={atToken} onChange={e => setAtToken(e.target.value)} placeholder="patXXXXXXXXXXXXXXXX.xxxxxxxxxxxxxxxx..." />
          </FieldGroup>
          <FieldGroup>
            <FieldRow>
              <FieldLabel>IDs des bases</FieldLabel>
              <FieldHint>dans l'URL — séparés par des virgules</FieldHint>
            </FieldRow>
            <Input type="text" value={atBases} onChange={e => setAtBases(e.target.value)} placeholder="appXXXXXXXXXXXXXX, appYYYYYYYYYYYYYY" />
          </FieldGroup>
          {atOk && <SuccessBanner><icons.CheckCircle2 size={15} /> Airtable configuré ! Redémarre le daemon.</SuccessBanner>}
          {atMut.isError && <ErrorBanner>Erreur : {String(atMut.error)}</ErrorBanner>}
          <SaveButton onClick={() => atMut.mutate()} disabled={!atToken || !atBases || atMut.isPending}>
            <icons.Save size={14} />
            {atMut.isPending ? 'Sauvegarde...' : 'Sauvegarder'}
          </SaveButton>
        </CardBody>
      </Card>
      ── fin GitLab + Airtable ─────────────────────────────────────────────── */}

      {/* ── Notes locales — désactivé temporairement (non prioritaire) ───────
      <SectionTitle>Notes locales</SectionTitle>
      <Card>
        <CardHeader>
          <CardTitle>Obsidian</CardTitle>
          <StatusPill $ok={config?.obsidian?.configured}>
            {config?.obsidian?.configured ? `✓ ${config.obsidian.display ?? 'Configuré'}` : 'Non configuré'}
          </StatusPill>
        </CardHeader>
        <CardBody>
          <FieldGroup>
            <FieldRow>
              <FieldLabel>Chemin du vault</FieldLabel>
              <FieldHint>dossier racine de votre vault Obsidian</FieldHint>
            </FieldRow>
            <Input type="text" value={obsPath} onChange={e => setObsPath(e.target.value)} placeholder="~/Documents/MyVault" />
          </FieldGroup>
          {obsOk && <SuccessBanner><icons.CheckCircle2 size={15} /> Obsidian configuré ! Vos notes seront indexées toutes les 5 min.</SuccessBanner>}
          {obsMut.isError && <ErrorBanner>Erreur : {String(obsMut.error)}</ErrorBanner>}
          <SaveButton onClick={() => obsMut.mutate()} disabled={!obsPath || obsMut.isPending}>
            <icons.Save size={14} />
            {obsMut.isPending ? 'Sauvegarde...' : 'Sauvegarder'}
          </SaveButton>
        </CardBody>
      </Card>
      ── fin Obsidian ───────────────────────────────────────────────────────── */}

      {/* ── Démarrage automatique ─────────────────────────────────────────── */}
      <InfoBox>
        <InfoTitle>Démarrage automatique au login</InfoTitle>
        <InfoDesc>
          Lance cette commande une seule fois pour que OSMOzzz démarre automatiquement à chaque connexion.
        </InfoDesc>
        <CodeBlock>osmozzz install</CodeBlock>
      </InfoBox>
    </Page>
  )
}
