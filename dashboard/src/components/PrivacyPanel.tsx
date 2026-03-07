import styled from 'styled-components'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { api } from '../api'
import type { PrivacyConfig } from '../api'

// ─── Styles ──────────────────────────────────────────────────────────────────

const Panel = styled.div`
  background: #fff;
  border: 1px solid #e8eaed;
  border-radius: 14px;
  overflow: hidden;
  box-shadow: 0 1px 3px rgba(0,0,0,.05);
`

const Header = styled.div`
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 20px 24px 16px;
  border-bottom: 1px solid #f3f4f6;
`

const HeaderLeft = styled.div`
  display: flex;
  align-items: center;
  gap: 10px;
`

const Title = styled.h3`
  font-size: 14px;
  font-weight: 600;
  color: #1a1d23;
`

const Subtitle = styled.p`
  font-size: 12px;
  color: #6b7280;
  margin-top: 2px;
`

const Badge = styled.span<{ $count: number }>`
  font-size: 11px;
  font-weight: 600;
  padding: 3px 10px;
  border-radius: 20px;
  background: ${({ $count }) => $count > 0 ? '#ede9fe' : '#f3f4f6'};
  color: ${({ $count }) => $count > 0 ? '#5b21b6' : '#6b7280'};
`

const Body = styled.div`
  display: flex;
  flex-direction: column;
`

const Row = styled.div`
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 24px;
  border-bottom: 1px solid #f9fafb;

  &:last-child {
    border-bottom: none;
  }
`

const RowLeft = styled.div``

const RowLabel = styled.p`
  font-size: 13px;
  font-weight: 500;
  color: #1a1d23;
`

const RowExample = styled.p`
  font-size: 11px;
  color: #9ca3af;
  margin-top: 2px;
  font-family: 'SF Mono', Monaco, monospace;
`

const Toggle = styled.button<{ $on: boolean }>`
  position: relative;
  width: 40px;
  height: 22px;
  border-radius: 11px;
  border: none;
  cursor: pointer;
  background: ${({ $on }) => $on ? '#5b5ef4' : '#d1d5db'};
  transition: background .2s;
  flex-shrink: 0;

  &::after {
    content: '';
    position: absolute;
    top: 3px;
    left: ${({ $on }) => $on ? '21px' : '3px'};
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #fff;
    transition: left .2s;
    box-shadow: 0 1px 3px rgba(0,0,0,.2);
  }
`

const DefaultTag = styled.span`
  font-size: 10px;
  font-weight: 500;
  color: #9ca3af;
  margin-left: 6px;
  text-transform: uppercase;
  letter-spacing: .04em;
`

// ─── Données des règles ───────────────────────────────────────────────────────

type RuleKey = keyof PrivacyConfig

interface RuleMeta {
  key: RuleKey
  label: string
  example: string
  defaultOn: boolean
}

const RULES: RuleMeta[] = [
  {
    key: 'credit_card',
    label: 'Numéros de carte bancaire',
    example: '4111 1111 1111 1111  →  [CB masquée]',
    defaultOn: true,
  },
  {
    key: 'iban',
    label: 'IBAN / RIB',
    example: 'FR76 3000 6000 0112 3456  →  [IBAN masqué]',
    defaultOn: true,
  },
  {
    key: 'api_keys',
    label: 'Clés API',
    example: 'sk-abc123...  →  [CLÉ API masquée]',
    defaultOn: true,
  },
  {
    key: 'email',
    label: 'Adresses email',
    example: 'jean@exemple.com  →  [email masqué]',
    defaultOn: false,
  },
  {
    key: 'phone',
    label: 'Numéros de téléphone',
    example: '06 12 34 56 78  →  [téléphone masqué]',
    defaultOn: false,
  },
]

// ─── Composant ────────────────────────────────────────────────────────────────

export function PrivacyPanel() {
  const qc = useQueryClient()

  const { data: config } = useQuery({
    queryKey: ['privacy'],
    queryFn: api.getPrivacy,
  })

  const mutation = useMutation({
    mutationFn: api.setPrivacy,
    onSuccess: () => qc.invalidateQueries({ queryKey: ['privacy'] }),
  })

  const toggle = (key: RuleKey) => {
    if (!config) return
    const next = { ...config, [key]: !config[key] }
    mutation.mutate(next)
  }

  const activeCount = config
    ? Object.values(config).filter(Boolean).length
    : 0

  return (
    <Panel>
      <Header>
        <HeaderLeft>
          <div>
            <Title>Pare-feu de confidentialité</Title>
            <Subtitle>Ce qui est activé sera masqué dans les réponses de Claude. Tes données locales restent intactes.</Subtitle>
          </div>
        </HeaderLeft>
        <Badge $count={activeCount}>
          {activeCount} filtre{activeCount !== 1 ? 's' : ''} actif{activeCount !== 1 ? 's' : ''}
        </Badge>
      </Header>
      <Body>
        {RULES.map(rule => (
          <Row key={rule.key}>
            <RowLeft>
              <RowLabel>
                {rule.label}
                {rule.defaultOn && <DefaultTag>défaut</DefaultTag>}
              </RowLabel>
              <RowExample>{rule.example}</RowExample>
            </RowLeft>
            <Toggle
              $on={config?.[rule.key] ?? rule.defaultOn}
              onClick={() => toggle(rule.key)}
              disabled={mutation.isPending}
              aria-label={`Activer/désactiver ${rule.label}`}
            />
          </Row>
        ))}
      </Body>
    </Panel>
  )
}
