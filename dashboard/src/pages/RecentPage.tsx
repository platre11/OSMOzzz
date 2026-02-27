import { useState, useEffect, useRef } from 'react'
import styled, { keyframes } from 'styled-components'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '../api'
import type { RecentDoc, ContactItem, MessageItem } from '../api'
import BlacklistPanel, { BannisBtn } from '../components/BlacklistPanel'

const CLICKABLE_SOURCES = new Set(['file', 'imessage', 'notes', 'calendar', 'terminal', 'chrome', 'safari'])

function resolveUrl(source: string, doc: RecentDoc): string {
  if (source === 'imessage') {
    const match = doc.title?.match(/([+\d]{7,})/)
    if (match) return `sms://${match[1]}`
  }
  return doc.url
}

function handleClick(source: string, doc: RecentDoc) {
  const url = resolveUrl(source, doc)
  if (source === 'chrome' || source === 'safari') {
    window.open(url, '_blank', 'noreferrer')
  } else {
    api.open(url)
  }
}

const SOURCE_LABELS: Record<string, string> = {
  email: 'Gmail', chrome: 'Chrome', file: 'Fichiers', imessage: 'iMessage',
  safari: 'Safari', notes: 'Notes', terminal: 'Terminal', calendar: 'Calendrier',
}
const SOURCE_COLORS: Record<string, string> = {
  email: '#dc2626', chrome: '#1d4ed8', file: '#16a34a', imessage: '#9333ea',
  safari: '#ea580c', notes: '#ca8a04', terminal: '#475569', calendar: '#0d9488',
}
const SOURCE_BG: Record<string, string> = {
  email: '#fef2f2', chrome: '#eff6ff', file: '#f0fdf4', imessage: '#faf5ff',
  safari: '#fff7ed', notes: '#fefce8', terminal: '#f8fafc', calendar: '#f0fdfa',
}
const SOURCES = ['email', 'chrome', 'file', 'imessage', 'safari', 'notes', 'terminal', 'calendar']

const spin = keyframes`to { transform: rotate(360deg); }`

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

const TabRow = styled.div`
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
`

const Tab = styled.button<{ $active?: boolean }>`
  padding: 6px 14px;
  border-radius: 8px;
  font-size: 12px;
  font-weight: 500;
  border: none;
  cursor: pointer;
  transition: all .15s;
  background: ${({ $active }) => $active ? '#5b5ef4' : '#fff'};
  color: ${({ $active }) => $active ? '#fff' : '#6b7280'};
  border: 1px solid ${({ $active }) => $active ? '#5b5ef4' : '#e5e7eb'};

  &:hover {
    background: ${({ $active }) => $active ? '#4a4de3' : '#f3f4f6'};
  }
`

const DocList = styled.div`
  display: flex;
  flex-direction: column;
  gap: 10px;
`

const Card = styled.div<{ $clickable?: boolean }>`
  background: #fff;
  border: 1px solid #e8eaed;
  border-radius: 14px;
  padding: 16px 20px;
  box-shadow: 0 1px 3px rgba(0,0,0,.05);
  cursor: ${({ $clickable }) => $clickable ? 'pointer' : 'default'};
  transition: box-shadow .15s;

  &:hover {
    box-shadow: ${({ $clickable }) => $clickable ? '0 4px 14px rgba(0,0,0,.10)' : '0 1px 3px rgba(0,0,0,.05)'};
  }
`

const CardTop = styled.div`
  display: flex;
  align-items: center;
  gap: 10px;
`

const Badge = styled.span<{ $source: string }>`
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: .04em;
  padding: 3px 8px;
  border-radius: 6px;
  flex-shrink: 0;
  background: ${({ $source }) => SOURCE_BG[$source] ?? '#f3f4f6'};
  color: ${({ $source }) => SOURCE_COLORS[$source] ?? '#374151'};
`

const DocTitle = styled.p`
  font-size: 13px;
  font-weight: 600;
  color: #1a1d23;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
  min-width: 0;
`

const DocContent = styled.p`
  font-size: 12px;
  color: #6b7280;
  line-height: 1.5;
  margin-top: 6px;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
`

const Pagination = styled.div`
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
`

const PageBtn = styled.button`
  padding: 7px 16px;
  border-radius: 8px;
  font-size: 13px;
  font-weight: 500;
  border: 1px solid #e5e7eb;
  background: #fff;
  color: #374151;
  cursor: pointer;
  transition: all .15s;

  &:hover:not(:disabled) { background: #f3f4f6; }
  &:disabled { opacity: .35; cursor: not-allowed; }
`

const PageNum = styled.span`
  font-size: 13px;
  color: #9ca3af;
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

const CardWrapper = styled.div`
  position: relative;
  &:hover > div:last-child { display: flex; }
`

const CardActions = styled.div`
  position: absolute;
  top: 10px;
  right: 12px;
  display: none;
  gap: 6px;
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

// ─── iMessage two-panel ───────────────────────────────────────────────────────

const ImessageLayout = styled.div`
  display: flex;
  gap: 0;
  height: 600px;
  border: 1px solid #e8eaed;
  border-radius: 14px;
  overflow: hidden;
  background: #fff;
`

const ContactPanel = styled.div`
  width: 180px;
  flex-shrink: 0;
  border-right: 1px solid #e8eaed;
  overflow-y: auto;
  background: #f8fafc;
`

const ContactItem = styled.div<{ $active?: boolean }>`
  padding: 12px 14px;
  cursor: pointer;
  border-bottom: 1px solid #f1f3f4;
  background: ${({ $active }) => $active ? '#ede9fe' : 'transparent'};
  transition: background .12s;

  &:hover {
    background: ${({ $active }) => $active ? '#ede9fe' : '#f1f3f4'};
  }
`

const ContactPhone = styled.p`
  font-size: 12px;
  font-weight: 600;
  color: #1a1d23;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
`

const ContactPreview = styled.p`
  font-size: 11px;
  color: #9ca3af;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  margin-top: 2px;
`

const ContactCount = styled.span`
  font-size: 10px;
  color: #c4b5fd;
  font-weight: 500;
`

const ConversationPanel = styled.div`
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
`

const ConvHeader = styled.div`
  padding: 12px 16px;
  border-bottom: 1px solid #e8eaed;
  font-size: 13px;
  font-weight: 600;
  color: #1a1d23;
  background: #fff;
  flex-shrink: 0;
`

const MessageList = styled.div`
  flex: 1;
  overflow-y: auto;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 8px;
`

const MessageRow = styled.div<{ $isMe: boolean }>`
  display: flex;
  justify-content: ${({ $isMe }) => $isMe ? 'flex-end' : 'flex-start'};
`

const Bubble = styled.div<{ $isMe: boolean }>`
  max-width: 72%;
  padding: 8px 12px;
  border-radius: ${({ $isMe }) => $isMe ? '16px 16px 4px 16px' : '16px 16px 16px 4px'};
  background: ${({ $isMe }) => $isMe ? '#9333ea' : '#f1f3f4'};
  color: ${({ $isMe }) => $isMe ? '#fff' : '#1a1d23'};
  font-size: 12px;
  line-height: 1.5;
  word-break: break-word;
`

const BubbleDate = styled.p<{ $isMe: boolean }>`
  font-size: 10px;
  color: #9ca3af;
  margin-top: 2px;
  text-align: ${({ $isMe }) => $isMe ? 'right' : 'left'};
`

const NoConvMsg = styled.div`
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #d1d5db;
  font-size: 13px;
`

function ImessageView() {
  const [selectedPhone, setSelectedPhone] = useState<string | null>(null)
  const bottomRef = useRef<HTMLDivElement>(null)

  const { data: contacts, isLoading: loadingContacts } = useQuery<ContactItem[]>({
    queryKey: ['imessage-contacts'],
    queryFn: () => api.getImessageContacts(),
    refetchInterval: false,
  })

  const { data: messages, isLoading: loadingMessages } = useQuery<MessageItem[]>({
    queryKey: ['imessage-conv', selectedPhone],
    queryFn: () => api.getImessageConversation(selectedPhone!, 200),
    enabled: !!selectedPhone,
    refetchInterval: false,
  })

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [messages])

  return (
    <ImessageLayout>
      <ContactPanel>
        {loadingContacts && <Loader style={{ margin: '20px auto' }} />}
        {contacts?.map(c => (
          <ContactItem
            key={c.phone}
            $active={selectedPhone === c.phone}
            onClick={() => setSelectedPhone(c.phone)}
          >
            <ContactPhone>{c.phone}</ContactPhone>
            <ContactPreview>{c.last_message}</ContactPreview>
            <ContactCount>{c.count} msg</ContactCount>
          </ContactItem>
        ))}
      </ContactPanel>

      <ConversationPanel>
        {selectedPhone
          ? <>
              <ConvHeader>{selectedPhone}</ConvHeader>
              <MessageList>
                {loadingMessages && <Loader style={{ margin: '20px auto' }} />}
                {messages?.map((m, i) => (
                  <div key={i}>
                    <MessageRow $isMe={m.is_me}>
                      <Bubble $isMe={m.is_me}>{m.text}</Bubble>
                    </MessageRow>
                    {m.date && <BubbleDate $isMe={m.is_me}>{m.date}</BubbleDate>}
                  </div>
                ))}
                <div ref={bottomRef} />
              </MessageList>
            </>
          : <NoConvMsg>Sélectionne un contact à gauche</NoConvMsg>
        }
      </ConversationPanel>
    </ImessageLayout>
  )
}


function BanCardItem({ doc, clickable, identifier, onBanned }: {
  doc: RecentDoc; clickable: boolean; identifier: string | null; onBanned: () => void
}) {
  const [menuOpen, setMenuOpen] = useState(false)

  async function doBan(kind: 'url' | 'source') {
    setMenuOpen(false)
    if (kind === 'url') await api.banUrl(doc.url)
    else if (identifier) await api.banSourceItem(doc.source, identifier)
    onBanned()
  }

  return (
    <CardWrapper>
      <Card
        $clickable={clickable}
        onClick={clickable ? () => handleClick(doc.source, doc) : undefined}
      >
        <CardTop>
          <Badge $source={doc.source}>{doc.source}</Badge>
          <DocTitle>{doc.title || doc.url}</DocTitle>
        </CardTop>
        <DocContent>{doc.content}</DocContent>
      </Card>
      <CardActions>
        <BanBtn onClick={e => { e.stopPropagation(); setMenuOpen(v => !v) }}>⊘ Bannir</BanBtn>
        {menuOpen && (
          <BanMenu>
            <BanMenuItem onClick={() => doBan('url')}>Ce document uniquement</BanMenuItem>
            {identifier && (
              <BanMenuItem onClick={() => doBan('source')}>Tout de : {identifier}</BanMenuItem>
            )}
          </BanMenu>
        )}
      </CardActions>
    </CardWrapper>
  )
}

function extractIdentifier(doc: RecentDoc): string | null {
  if (doc.source === 'imessage') {
    const m = (doc.title ?? '').match(/([+\d]{7,})/)
    return m ? m[1] : null
  }
  if (doc.source === 'chrome' || doc.source === 'safari') {
    try { return new URL(doc.url).hostname } catch { return null }
  }
  if (doc.source === 'file') {
    const parts = doc.url.split('/'); parts.pop(); return parts.join('/') || null
  }
  if (doc.source === 'email') {
    const m = doc.content?.match(/^De\s*:\s*(.+)/m)
    return m ? m[1].trim() : null
  }
  return null
}

export default function RecentPage() {
  const [source, setSource]         = useState('email')
  const [page, setPage]             = useState(0)
  const [showBannis, setShowBannis] = useState(false)
  const limit = 20
  const queryClient = useQueryClient()

  const { data, isLoading } = useQuery<RecentDoc[]>({
    queryKey: ['recent', source, page],
    queryFn:  () => api.getRecent(source, limit, page * limit),
    enabled:  source !== 'imessage',
    refetchInterval: false,
  })

  return (
    <Page>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <PageTitle>Documents récents</PageTitle>
        <BannisBtn onClick={() => setShowBannis(true)}>⊘ Bannis</BannisBtn>
      </div>

      {showBannis && (
        <BlacklistPanel source={source} onClose={() => setShowBannis(false)} />
      )}

      <TabRow>
        {SOURCES.map(s => (
          <Tab key={s} $active={source === s} onClick={() => { setSource(s); setPage(0) }}>
            {SOURCE_LABELS[s]}
          </Tab>
        ))}
      </TabRow>

      {source === 'imessage'
        ? <ImessageView />
        : <>
            {isLoading && <Loader />}
            {!isLoading && data?.length === 0 && <EmptyMsg>Aucun document dans cette source.</EmptyMsg>}

            <DocList>
              {data?.map((doc, i) => {
                const clickable = CLICKABLE_SOURCES.has(doc.source)
                const identifier = extractIdentifier(doc)
                return (
                  <BanCardItem
                    key={i}
                    doc={doc}
                    clickable={clickable}
                    identifier={identifier}
                    onBanned={() => queryClient.invalidateQueries({ queryKey: ['recent', source, page] })}
                  />
                )
              })}
            </DocList>

            {data && data.length > 0 && (
              <Pagination>
                <PageBtn onClick={() => setPage(p => p - 1)} disabled={page === 0}>← Précédent</PageBtn>
                <PageNum>Page {page + 1}</PageNum>
                <PageBtn onClick={() => setPage(p => p + 1)} disabled={data.length < limit}>Suivant →</PageBtn>
              </Pagination>
            )}
          </>
      }
    </Page>
  )
}
