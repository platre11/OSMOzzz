import { useState } from 'react'
import styled from 'styled-components'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { api } from '../api'
import type { ConfigData } from '../api'
import { icons } from '../lib/assets'

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
  background: ${({ $ok }) => $ok ? '#d1fae5' : '#fee2e2'};
  color: ${({ $ok }) => $ok ? '#065f46' : '#991b1b'};
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

export default function ConfigPage() {
  const qc = useQueryClient()
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [success, setSuccess]   = useState(false)

  const { data: config } = useQuery<ConfigData>({
    queryKey: ['config'],
    queryFn:  api.getConfig,
  })

  const mutation = useMutation({
    mutationFn: () => api.saveGmail(username, password),
    onSuccess: () => {
      setSuccess(true)
      setPassword('')
      qc.invalidateQueries({ queryKey: ['config'] })
      setTimeout(() => setSuccess(false), 4000)
    },
  })

  return (
    <Page>
      <PageTitle>Configuration</PageTitle>

      <Card>
        <CardHeader>
          <CardTitle>Gmail IMAP</CardTitle>
          <StatusPill $ok={config?.gmail_configured}>
            {config?.gmail_configured ? `✓ ${config.gmail_username}` : 'Non configuré'}
          </StatusPill>
        </CardHeader>
        <CardBody>
          <FieldGroup>
            <FieldLabel>Adresse Gmail</FieldLabel>
            <Input
              type="email"
              value={username}
              onChange={e => setUsername(e.target.value)}
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
              value={password}
              onChange={e => setPassword(e.target.value)}
              placeholder="xxxx xxxx xxxx xxxx"
            />
          </FieldGroup>

          {success && (
            <SuccessBanner>
              <icons.CheckCircle2 size={15} />
              Gmail configuré ! Redémarre le daemon pour activer la sync.
            </SuccessBanner>
          )}

          {mutation.isError && (
            <ErrorBanner>Erreur : {String(mutation.error)}</ErrorBanner>
          )}

          <SaveButton
            onClick={() => mutation.mutate()}
            disabled={!username || !password || mutation.isPending}
          >
            <icons.Save size={14} />
            {mutation.isPending ? 'Sauvegarde...' : 'Sauvegarder'}
          </SaveButton>
        </CardBody>
      </Card>

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
