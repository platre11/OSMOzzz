import { useState, useCallback } from 'react'
import styled, { keyframes } from 'styled-components'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '../api'
import type { GroupedSearchResponse, SearchDoc } from '../api'
import { icons } from '../lib/assets'
import type { LucideIcon } from 'lucide-react'
import BlacklistPanel, { BannisBtn } from '../components/BlacklistPanel'
import { highlightText, renderEmailContent } from '../lib/highlight'

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

const CLICKABLE_SOURCES = new Set(['file', 'imessage', 'notes', 'calendar', 'terminal', 'chrome', 'safari'])

const spin = keyframes`to { transform: rotate(360deg); }`

// ─── Styled components ────────────────────────────────────────────────────────

const Page = styled.div`
  display: flex;
  flex-direction: column;
  gap: 24px;
`

const TopRow = styled.div`
  display: flex;
  align-items: center;
  gap: 10px;
`

const SearchWrapper = styled.div`
  position: relative;
  color: #9ca3af;
  flex: 1;

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

// ─── Filter bar ───────────────────────────────────────────────────────────────

const FilterSection = styled.div`
  display: flex;
  flex-direction: column;
  gap: 8px;
`


const DateRow = styled.div`
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
`

const DateGroup = styled.div<{ $active: boolean }>`
  display: flex;
  align-items: center;
  gap: 6px;
  background: #fff;
  border: 1.5px solid ${({ $active }) => $active ? '#5b5ef4' : '#e5e7eb'};
  border-radius: 10px;
  padding: 6px 10px;
  box-shadow: ${({ $active }) => $active ? '0 0 0 2px rgba(91,94,244,.12)' : 'none'};
  transition: all .15s;
`

const FilterLabel = styled.span`
  font-size: 11px;
  color: #9ca3af;
  white-space: nowrap;
`

const FilterDate = styled.input`
  border: none;
  outline: none;
  font-size: 12px;
  color: #1a1d23;
  background: transparent;
  cursor: pointer;
  font-family: inherit;
  &::-webkit-calendar-picker-indicator { opacity: 0.5; cursor: pointer; }
`

const ClearBtn = styled.button`
  display: flex;
  align-items: center;
  gap: 4px;
  background: #f3f4f6;
  border: 1px solid #e5e7eb;
  border-radius: 8px;
  padding: 5px 10px;
  font-size: 11px;
  color: #6b7280;
  cursor: pointer;
  &:hover { background: #e5e7eb; }
`


// ─── Brut results ─────────────────────────────────────────────────────────────

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


const CardActions = styled.div`
  position: absolute;
  top: 10px;
  right: 12px;
  display: none;
  gap: 6px;
`

const CardWrapper = styled.div`
  position: relative;
  &:hover ${CardActions} { display: flex; }
`

const BanBtn = styled.button`
  font-size: 10px;
  font-weight: 600;
  padding: 3px 8px;
  border-radius: 6px;
  border: 1px solid #fca5a5;
  background: #fef2f2;
  color: #dc2626;
  cursor: pointer;
  white-space: nowrap;
  &:hover { background: #fee2e2; }
`

const BanMenu = styled.div`
  position: absolute;
  top: 28px;
  right: 0;
  background: #fff;
  border: 1px solid #e8eaed;
  border-radius: 10px;
  box-shadow: 0 8px 24px rgba(0,0,0,.12);
  z-index: 10;
  min-width: 220px;
  overflow: hidden;
`

const BanMenuItem = styled.button`
  display: block;
  width: 100%;
  text-align: left;
  padding: 10px 14px;
  font-size: 12px;
  color: #374151;
  background: none;
  border: none;
  cursor: pointer;
  border-bottom: 1px solid #f3f4f6;
  &:last-child { border-bottom: none; }
  &:hover { background: #fef2f2; color: #dc2626; }
`

// ─── Helpers ──────────────────────────────────────────────────────────────────

function resolveUrl(source: string, doc: SearchDoc): string {
  if (source === 'imessage') {
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

function extractSourceIdentifier(source: string, doc: SearchDoc): string | null {
  if (source === 'imessage') {
    const m = (doc.title ?? '').match(/([+\d]{7,})/)
    return m ? m[1] : null
  }
  if (source === 'chrome' || source === 'safari') {
    try { return new URL(doc.url).hostname } catch { return null }
  }
  if (source === 'file') {
    const parts = doc.url.split('/')
    parts.pop()
    return parts.join('/') || null
  }
  if (source === 'email') {
    const m = doc.content?.match(/^From:\s*(.+)/m)
    return m ? m[1].trim() : null
  }
  return null
}

function sourceIdentifierLabel(source: string): string {
  return { email: 'expéditeur', imessage: 'contact', chrome: 'domaine', safari: 'domaine', file: 'dossier' }[source] ?? 'source'
}

function ResultCard({ doc, source, query, onBanned }: { doc: SearchDoc; source: string; query: string; onBanned: () => void }) {
  const [menuOpen, setMenuOpen] = useState(false)
  const clickable = CLICKABLE_SOURCES.has(source)
  const identifier = extractSourceIdentifier(source, doc)

  async function doBan(kind: 'url' | 'source') {
    setMenuOpen(false)
    if (kind === 'url') {
      await api.banUrl(doc.url)
    } else if (identifier) {
      await api.banSourceItem(source, identifier)
    }
    onBanned()
  }

  return (
    <CardWrapper>
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
      <CardActions>
        <BanBtn onClick={e => { e.stopPropagation(); setMenuOpen(v => !v) }}>Ban</BanBtn>
        {menuOpen && (
          <BanMenu>
            <BanMenuItem onClick={() => doBan('url')}>Ce document uniquement</BanMenuItem>
            {identifier && (
              <BanMenuItem onClick={() => doBan('source')}>
                Tout cet {sourceIdentifierLabel(source)} : {identifier}
              </BanMenuItem>
            )}
          </BanMenu>
        )}
      </CardActions>
    </CardWrapper>
  )
}

// ─── Page ─────────────────────────────────────────────────────────────────────

export default function SearchPage() {
  const [query, setQuery]                     = useState('')
  const [debouncedQuery, setDebounced]        = useState('')
  const [showBannis, setShowBannis]           = useState(false)
  const [filterFrom, setFilterFrom]           = useState('')
  const [filterTo,   setFilterTo]             = useState('')
  const queryClient = useQueryClient()

  const debounce = useCallback((value: string) => {
    const t = setTimeout(() => setDebounced(value), 300)
    return () => clearTimeout(t)
  }, [])

  const handleChange = (v: string) => { setQuery(v); debounce(v) }

  const hasActiveFilters = !!filterFrom || !!filterTo
  const clearFilters = () => { setFilterFrom(''); setFilterTo('') }

  const { data, isLoading } = useQuery<GroupedSearchResponse>({
    queryKey: ['search', debouncedQuery, filterFrom, filterTo],
    queryFn:  () => api.search(debouncedQuery, {
      from: filterFrom || undefined,
      to:   filterTo   || undefined,
    }),
    enabled:  debouncedQuery.length > 1,
    refetchInterval: false,
  })

  const filteredGroups = data?.groups ?? []
  const hasResults = filteredGroups.length > 0

  return (
    <Page>
      {showBannis && <BlacklistPanel source="all" onClose={() => setShowBannis(false)} />}

      <TopRow>
        <SearchWrapper>
          <icons.Search size={16} />
          <SearchInput
            type="text"
            value={query}
            onChange={e => handleChange(e.target.value)}
            placeholder="Cherche dans toute ta memoire..."
            autoFocus
          />
        </SearchWrapper>
        <BannisBtn onClick={() => setShowBannis(true)}>Ban</BannisBtn>
      </TopRow>

      <FilterSection>
        <DateRow>
          <DateGroup $active={!!filterFrom}>
            <FilterLabel>Du</FilterLabel>
            <FilterDate type="date" value={filterFrom} onChange={e => setFilterFrom(e.target.value)} />
          </DateGroup>

          <DateGroup $active={!!filterTo}>
            <FilterLabel>Au</FilterLabel>
            <FilterDate type="date" value={filterTo} onChange={e => setFilterTo(e.target.value)} />
          </DateGroup>

          {hasActiveFilters && (
            <ClearBtn onClick={clearFilters}>✕ Effacer les filtres</ClearBtn>
          )}
        </DateRow>
      </FilterSection>

      {isLoading && <Loader />}

      {!isLoading && debouncedQuery.length > 1 && !hasResults && (
        <EmptyMsg>Aucun resultat pour « {debouncedQuery} »{hasActiveFilters ? ' avec ces filtres' : ''}</EmptyMsg>
      )}

      {!debouncedQuery && (
        <HintMsg>Tape un mot-cle pour chercher dans tes emails, messages, fichiers...</HintMsg>
      )}

      {hasResults && (
        <GroupsContainer>
          {filteredGroups.map(group => {
            const Icon = SOURCE_ICONS[group.source]
            return (
              <Group key={group.source}>
                <GroupHeader $source={group.source}>
                  <GroupIcon>{Icon && <Icon size={14} />}</GroupIcon>
                  <GroupTitle>{SOURCE_LABELS[group.source] ?? group.source}</GroupTitle>
                  <GroupCount>{group.results.length} resultat{group.results.length > 1 ? 's' : ''}</GroupCount>
                </GroupHeader>
                <CardList>
                  {group.results.map((doc, i) => (
                    <ResultCard
                      key={i}
                      doc={doc}
                      source={group.source}
                      query={debouncedQuery}
                      onBanned={() => queryClient.invalidateQueries({ queryKey: ['search', debouncedQuery] })}
                    />
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
