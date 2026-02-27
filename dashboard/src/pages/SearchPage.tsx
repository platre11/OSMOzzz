import { useState, useCallback } from 'react'
import styled, { keyframes } from 'styled-components'
import { useQuery } from '@tanstack/react-query'
import { api } from '../api'
import type { GroupedSearchResponse, SearchDoc } from '../api'
import { icons } from '../lib/assets'
import type { LucideIcon } from 'lucide-react'

const SOURCE_LABELS: Record<string, string> = {
  email: 'Gmail', chrome: 'Chrome', file: 'Fichiers', imessage: 'iMessage',
  safari: 'Safari', notes: 'Notes', terminal: 'Terminal', calendar: 'Calendrier',
}
const SOURCE_COLORS: Record<string, string> = {
  email: '#dc2626', chrome: '#1d4ed8', file: '#16a34a', imessage: '#9333ea',
  safari: '#ea580c', notes: '#ca8a04', terminal: '#475569', calendar: '#0d9488',
}
const SOURCE_ICONS: Record<string, LucideIcon> = {
  email: icons.Mail, chrome: icons.Chrome, file: icons.FileText,
  imessage: icons.MessageCircle, safari: icons.Globe, notes: icons.BookOpen,
  terminal: icons.Terminal, calendar: icons.Calendar,
}

// Sources dont toute la card est cliquable
const CLICKABLE_SOURCES = new Set(['file', 'imessage', 'notes', 'calendar', 'terminal', 'chrome', 'safari'])

// Regex pour détecter URLs et emails dans le contenu
const LINK_RE = /(https?:\/\/[^\s<>"'[\]()]+|[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,})/g

const spin = keyframes`to { transform: rotate(360deg); }`

// ─── Styled components ────────────────────────────────────────────────────────

const Page = styled.div`
  display: flex;
  flex-direction: column;
  gap: 32px;
`

const SearchWrapper = styled.div`
  position: relative;
  color: #9ca3af;

  svg {
    position: absolute;
    left: 14px;
    top: 50%;
    transform: translateY(-50%);
    pointer-events: none;
  }
`

const SearchInput = styled.input`
  width: 100%;
  font-size: 15px;
  font-family: inherit;
  padding: 13px 16px 13px 44px;
  background: #fff;
  border: 1px solid #e5e7eb;
  border-radius: 12px;
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

const Loader = styled.div`
  width: 20px;
  height: 20px;
  border: 2px solid #e5e7eb;
  border-top-color: #5b5ef4;
  border-radius: 50%;
  animation: ${spin} .7s linear infinite;
  margin: 60px auto;
`

const EmptyMsg = styled.p`
  text-align: center;
  padding: 60px;
  color: #9ca3af;
`

const HintMsg = styled.p`
  text-align: center;
  padding: 60px;
  color: #d1d5db;
  font-size: 14px;
`

const GroupsContainer = styled.div`
  display: flex;
  flex-direction: column;
  gap: 28px;
`

const Group = styled.div`
  display: flex;
  flex-direction: column;
  gap: 10px;
`

const GroupHeader = styled.div<{ $source: string }>`
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 2px;
  color: ${({ $source }) => SOURCE_COLORS[$source] ?? '#374151'};
`

const GroupIcon = styled.div`display: flex; align-items: center;`

const GroupTitle = styled.h3`
  font-size: 13px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: .06em;
`

const GroupCount = styled.span`
  font-size: 11px;
  font-weight: 500;
  color: #9ca3af;
  margin-left: 2px;
`

const CardList = styled.div`
  display: flex;
  flex-direction: column;
  gap: 8px;
`

const Card = styled.div<{ $clickable?: boolean }>`
  background: #fff;
  border: 1px solid #e8eaed;
  border-radius: 12px;
  padding: 14px 18px;
  box-shadow: 0 1px 3px rgba(0,0,0,.04);
  transition: box-shadow .15s;
  cursor: ${({ $clickable }) => $clickable ? 'pointer' : 'default'};

  &:hover {
    box-shadow: ${({ $clickable }) => $clickable ? '0 4px 14px rgba(0,0,0,.10)' : '0 1px 3px rgba(0,0,0,.04)'};
  }
`

const CardHeader = styled.div`
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 6px;
`

const CardTitle = styled.p`
  font-size: 13px;
  font-weight: 600;
  color: #1a1d23;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
  min-width: 0;
`

const CardDate = styled.span`
  font-size: 11px;
  color: #9ca3af;
  flex-shrink: 0;
`

const CardContent = styled.p`
  font-size: 12px;
  color: #6b7280;
  line-height: 1.6;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
`

const CardUrl = styled.code`
  display: block;
  font-size: 10px;
  color: #d1d5db;
  margin-top: 6px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
`

const Highlight = styled.mark`
  background: rgba(91, 94, 244, 0.18);
  color: inherit;
  border-radius: 3px;
  padding: 0 1px;
`

const InlineLink = styled.a`
  color: #5b5ef4;
  text-decoration: underline;
  text-underline-offset: 2px;
  word-break: break-all;

  &:hover { color: #4a4de3; }
`

// ─── Helpers ──────────────────────────────────────────────────────────────────

function highlightText(text: string, query: string): React.ReactNode {
  if (!query || query.length < 2) return text
  const idx = text.toLowerCase().indexOf(query.toLowerCase())
  if (idx === -1) return text
  return (
    <>
      {text.slice(0, idx)}
      <Highlight>{text.slice(idx, idx + query.length)}</Highlight>
      {highlightText(text.slice(idx + query.length), query)}
    </>
  )
}

// Pour les emails : rend le texte avec liens cliquables + surlignage
function renderEmailContent(text: string, query: string): React.ReactNode {
  const parts = text.split(LINK_RE)
  return (
    <>
      {parts.map((part, i) => {
        if (/^https?:\/\//.test(part)) {
          return (
            <InlineLink key={i} href={part} target="_blank" rel="noreferrer" onClick={e => e.stopPropagation()}>
              {highlightText(part, query)}
            </InlineLink>
          )
        }
        if (/^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$/.test(part)) {
          return (
            <InlineLink key={i} href={`mailto:${part}`} onClick={e => e.stopPropagation()}>
              {highlightText(part, query)}
            </InlineLink>
          )
        }
        return <span key={i}>{highlightText(part, query)}</span>
      })}
    </>
  )
}

function resolveUrl(source: string, doc: SearchDoc): string {
  if (source === 'imessage') {
    // Extrait le numéro depuis le titre "iMessage ← +33766300049"
    const match = doc.title?.match(/([+\d]{7,})/)
    if (match) return `sms://${match[1]}`
  }
  return doc.url
}

function handleCardClick(source: string, doc: SearchDoc) {
  const url = resolveUrl(source, doc)
  if (source === 'chrome' || source === 'safari') {
    window.open(url, '_blank', 'noreferrer')
  } else {
    api.open(url)
  }
}

// ─── ResultCard ───────────────────────────────────────────────────────────────

function ResultCard({ doc, source, query }: { doc: SearchDoc; source: string; query: string }) {
  const clickable = CLICKABLE_SOURCES.has(source)

  return (
    <Card
      $clickable={clickable}
      onClick={clickable ? () => handleCardClick(source, doc) : undefined}
    >
      <CardHeader>
        <CardTitle>{highlightText(doc.title || doc.url, query)}</CardTitle>
        {doc.date && <CardDate>{doc.date}</CardDate>}
      </CardHeader>
      {doc.content && (
        <CardContent>
          {source === 'email'
            ? renderEmailContent(doc.content, query)
            : highlightText(doc.content, query)
          }
        </CardContent>
      )}
      <CardUrl>{doc.url}</CardUrl>
    </Card>
  )
}

// ─── Page ─────────────────────────────────────────────────────────────────────

export default function SearchPage() {
  const [query, setQuery]              = useState('')
  const [debouncedQuery, setDebounced] = useState('')

  const debounce = useCallback((value: string) => {
    const t = setTimeout(() => setDebounced(value), 300)
    return () => clearTimeout(t)
  }, [])

  const handleChange = (v: string) => { setQuery(v); debounce(v) }

  const { data, isLoading } = useQuery<GroupedSearchResponse>({
    queryKey: ['search', debouncedQuery],
    queryFn:  () => api.search(debouncedQuery),
    enabled:  debouncedQuery.length > 1,
    refetchInterval: false,
  })

  const hasResults = (data?.groups?.length ?? 0) > 0

  return (
    <Page>
      <SearchWrapper>
        <icons.Search size={16} />
        <SearchInput
          type="text"
          value={query}
          onChange={e => handleChange(e.target.value)}
          placeholder="Cherche dans toute ta mémoire..."
          autoFocus
        />
      </SearchWrapper>

      {isLoading && <Loader />}

      {!isLoading && debouncedQuery.length > 1 && !hasResults && (
        <EmptyMsg>Aucun résultat pour « {debouncedQuery} »</EmptyMsg>
      )}

      {!debouncedQuery && (
        <HintMsg>Tape un mot-clé pour chercher dans tes emails, messages, fichiers…</HintMsg>
      )}

      {hasResults && (
        <GroupsContainer>
          {data!.groups.map(group => {
            const Icon = SOURCE_ICONS[group.source]
            return (
              <Group key={group.source}>
                <GroupHeader $source={group.source}>
                  <GroupIcon>{Icon && <Icon size={14} />}</GroupIcon>
                  <GroupTitle>{SOURCE_LABELS[group.source] ?? group.source}</GroupTitle>
                  <GroupCount>{group.results.length} résultat{group.results.length > 1 ? 's' : ''}</GroupCount>
                </GroupHeader>
                <CardList>
                  {group.results.map((doc, i) => (
                    <ResultCard key={i} doc={doc} source={group.source} query={debouncedQuery} />
                  ))}
                </CardList>
              </Group>
            )
          })}
        </GroupsContainer>
      )}
    </Page>
  )
}
