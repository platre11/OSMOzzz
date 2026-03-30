import { useState, useEffect, useRef } from 'react'
import styled, { keyframes } from 'styled-components'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '../api'
import type { RecentDoc, ContactItem, MessageItem, StatusData, SearchDoc, GroupedSearchResponse, BlacklistEntry } from '../api'
import { icons } from '../lib/assets'
import { highlightText, renderEmailContent } from '../lib/highlight'
import BlacklistPanel from '../components/BlacklistPanel'

const CLICKABLE_SOURCES = new Set(['file', 'imessage', 'notes', 'calendar', 'terminal', 'chrome', 'safari'])

function openUrl(source: string, url: string) {
  if (source === 'chrome' || source === 'safari') {
    window.open(url, '_blank', 'noreferrer')
  } else {
    api.open(url)
  }
}

const SOURCE_LABELS: Record<string, string> = {
  email: 'Gmail', chrome: 'Chrome', file: 'Fichiers', imessage: 'iMessage',
  safari: 'Safari', notes: 'Notes', terminal: 'Terminal', calendar: 'Calendrier',
  notion: 'Notion', github: 'GitHub', linear: 'Linear', jira: 'Jira',
  slack: 'Slack', trello: 'Trello', todoist: 'Todoist', gitlab: 'GitLab',
  airtable: 'Airtable', obsidian: 'Obsidian',
}
const SOURCE_COLORS: Record<string, string> = {
  email: '#dc2626', chrome: '#1d4ed8', file: '#16a34a', imessage: '#9333ea',
  safari: '#ea580c', notes: '#ca8a04', terminal: '#475569', calendar: '#0d9488',
  notion: '#000000', github: '#24292f', linear: '#5e6ad2', jira: '#0052cc',
  slack: '#4a154b', trello: '#0079bf', todoist: '#db4035', gitlab: '#e24329',
  airtable: '#18bfff', obsidian: '#7c3aed',
}
const SOURCE_BG: Record<string, string> = {
  email: '#fef2f2', chrome: '#eff6ff', file: '#f0fdf4', imessage: '#faf5ff',
  safari: '#fff7ed', notes: '#fefce8', terminal: '#f8fafc', calendar: '#f0fdfa',
  notion: '#f5f5f5', github: '#f6f8fa', linear: '#f0f0ff', jira: '#e6f0ff',
  slack: '#fdf4ff', trello: '#e6f4ff', todoist: '#fff0ef', gitlab: '#fff2ef',
  airtable: '#e6f9ff', obsidian: '#f5f0ff',
}
const SOURCES = [
  'email', 'chrome', 'file', 'imessage', 'safari', 'notes', 'terminal', 'calendar',
  'notion', 'github', 'linear', 'jira', 'slack', 'trello', 'todoist', 'gitlab', 'airtable', 'obsidian',
]

const spin = keyframes`to { transform: rotate(360deg); }`

// ─── Layout ───────────────────────────────────────────────────────────────────

const Page = styled.div`display: flex; flex-direction: column; gap: 24px;`

const PageTitle = styled.h1`
  font-size: 22px; font-weight: 700; color: #1a1d23; letter-spacing: -.02em;
`

const TabRow = styled.div`display: flex; gap: 6px; flex-wrap: wrap;`

const Tab = styled.button<{ $active?: boolean }>`
  padding: 6px 14px; border-radius: 8px; font-size: 12px; font-weight: 500;
  border: 1px solid ${({ $active }) => $active ? '#5b5ef4' : '#e5e7eb'};
  background: ${({ $active }) => $active ? '#5b5ef4' : '#fff'};
  color: ${({ $active }) => $active ? '#fff' : '#6b7280'};
  cursor: pointer; transition: all .15s;
  &:hover { background: ${({ $active }) => $active ? '#4a4de3' : '#f3f4f6'}; }
`

// ─── Toolbar ──────────────────────────────────────────────────────────────────

const ToolRow = styled.div`display: flex; align-items: flex-start; gap: 8px;`

const SearchInputWrap = styled.div`
  position: relative; flex: 1; display: flex; align-items: center; color: #9ca3af;
  svg { position: absolute; left: 14px; pointer-events: none; }
`

const SearchInputField = styled.input`
  width: 100%; box-sizing: border-box;
  font-size: 14px; font-family: inherit;
  padding: 10px 36px 10px 40px;
  background: #fff; border: 1px solid #e5e7eb; border-radius: 10px;
  color: #1a1d23; outline: none;
  transition: border-color .15s, box-shadow .15s;
  &:focus { border-color: #5b5ef4; box-shadow: 0 0 0 3px rgba(91,94,244,.12); }
  &::placeholder { color: #9ca3af; }
`

const ClearSearchBtn = styled.button`
  position: absolute; right: 12px; background: none; border: none;
  font-size: 12px; color: #9ca3af; cursor: pointer;
  &:hover { color: #374151; }
`

const DotsMenuWrap = styled.div`position: relative; flex-shrink: 0;`

const DotsBtn = styled.button<{ $active?: boolean }>`
  width: 38px; height: 38px; border-radius: 10px;
  border: 1px solid ${({ $active }) => $active ? '#5b5ef4' : '#e5e7eb'};
  background: ${({ $active }) => $active ? '#ededff' : '#fff'};
  color: ${({ $active }) => $active ? '#5b5ef4' : '#6b7280'};
  font-size: 18px; font-weight: 700; letter-spacing: 2px;
  cursor: pointer; display: flex; align-items: center; justify-content: center;
  transition: all .15s; line-height: 1; padding-bottom: 4px;
  &:hover { background: ${({ $active }) => $active ? '#ededff' : '#f3f4f6'}; }
`

const DotsDropdown = styled.div`
  position: absolute; top: calc(100% + 8px); right: 0;
  background: #fff; border: 1px solid #e8eaed; border-radius: 12px;
  box-shadow: 0 8px 24px rgba(0,0,0,.12); z-index: 50;
  min-width: 290px; padding: 14px;
  display: flex; flex-direction: column; gap: 10px;
`

const DateMenuRow = styled.div`
  display: flex; align-items: center; gap: 6px; flex-wrap: wrap;
`

const DateMenuLabel = styled.span`font-size: 11px; color: #9ca3af; white-space: nowrap;`

const DateMenuInput = styled.input`
  border: 1px solid #e5e7eb; border-radius: 7px; outline: none;
  font-size: 12px; color: #1a1d23; background: #f9fafb; font-family: inherit;
  padding: 5px 8px; cursor: pointer;
  &::-webkit-calendar-picker-indicator { opacity: 0.5; cursor: pointer; }
  &:focus { border-color: #5b5ef4; }
`

const ClearDateBtn = styled.button`
  padding: 4px 8px; border-radius: 6px; border: 1px solid #e5e7eb;
  background: #f3f4f6; font-size: 11px; color: #6b7280; cursor: pointer;
  &:hover { background: #e5e7eb; }
`

const MenuSep = styled.div`height: 1px; background: #f3f4f6;`

const BlacklistCard = styled.div`
  background: #fff; border: 1px solid #e8eaed; border-radius: 12px;
  padding: 12px 16px; display: flex; align-items: center; justify-content: space-between;
  box-shadow: 0 1px 2px rgba(0,0,0,.04);
`
const BlacklistCount = styled.span<{ $n: number }>`
  font-size: 11px; font-weight: 600; padding: 3px 10px; border-radius: 20px;
  background: ${({ $n }) => $n > 0 ? '#fee2e2' : '#f3f4f6'};
  color: ${({ $n }) => $n > 0 ? '#991b1b' : '#6b7280'};
  margin-right: 10px;
`
const ManageBtn = styled.button`
  padding: 6px 14px; border-radius: 8px; font-size: 12px; font-weight: 500;
  border: 1px solid #e5e7eb; background: #fff; color: #374151; cursor: pointer;
  transition: all .15s;
  &:hover { background: #f3f4f6; }
`

const BannisToggle = styled.button<{ $active: boolean }>`
  display: flex; align-items: center; gap: 8px; width: 100%; text-align: left;
  padding: 8px 10px; border-radius: 8px;
  border: 1px solid ${({ $active }) => $active ? '#dc2626' : '#f3f4f6'};
  background: ${({ $active }) => $active ? '#fef2f2' : 'transparent'};
  color: ${({ $active }) => $active ? '#dc2626' : '#374151'};
  font-size: 13px; font-weight: 500; font-family: inherit; cursor: pointer;
  transition: all .15s;
  &:hover { background: ${({ $active }) => $active ? '#fee2e2' : '#f9fafb'}; }
`

// ─── Content cards ────────────────────────────────────────────────────────────

const DocList = styled.div`display: flex; flex-direction: column; gap: 10px;`

const Card = styled.div<{ $clickable?: boolean }>`
  background: #fff; border: 1px solid #e8eaed; border-radius: 14px;
  padding: 16px 20px; box-shadow: 0 1px 3px rgba(0,0,0,.05);
  cursor: ${({ $clickable }) => $clickable ? 'pointer' : 'default'};
  transition: box-shadow .15s;
  &:hover { box-shadow: ${({ $clickable }) => $clickable ? '0 4px 14px rgba(0,0,0,.10)' : '0 1px 3px rgba(0,0,0,.05)'}; }
`

const CardTop = styled.div`display: flex; align-items: center; gap: 10px;`

const Badge = styled.span<{ $source: string }>`
  font-size: 10px; font-weight: 700; text-transform: uppercase; letter-spacing: .04em;
  padding: 3px 8px; border-radius: 6px; flex-shrink: 0;
  background: ${({ $source }) => SOURCE_BG[$source] ?? '#f3f4f6'};
  color: ${({ $source }) => SOURCE_COLORS[$source] ?? '#374151'};
`

const CardHeader = styled.div`
  display: flex; align-items: baseline; justify-content: space-between; gap: 12px; margin-bottom: 6px;
`

const DocTitle = styled.p`
  font-size: 13px; font-weight: 600; color: #1a1d23;
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap; flex: 1; min-width: 0;
`

const CardDate = styled.span`font-size: 11px; color: #9ca3af; flex-shrink: 0;`

const DocContent = styled.p`
  font-size: 12px; color: #6b7280; line-height: 1.5; margin-top: 6px;
  display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden;
`

const CardUrl = styled.code`
  display: block; font-size: 10px; color: #d1d5db; margin-top: 6px;
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
`

const CardWrapper = styled.div`
  position: relative;
  &:hover > div:last-child { display: flex; }
`

const CardActions = styled.div`
  position: absolute; top: 10px; right: 12px; display: none; gap: 6px;
`

const BanBtn = styled.button`
  font-size: 10px; font-weight: 600; padding: 3px 8px; border-radius: 6px;
  border: 1px solid #fca5a5; background: #fef2f2; color: #dc2626; cursor: pointer;
  &:hover { background: #fee2e2; }
`

const BanMenu = styled.div`
  position: absolute; top: 28px; right: 0; background: #fff;
  border: 1px solid #e8eaed; border-radius: 10px; box-shadow: 0 8px 24px rgba(0,0,0,.12);
  z-index: 10; min-width: 220px; overflow: hidden;
`

const BanMenuItem = styled.button`
  display: block; width: 100%; text-align: left; padding: 10px 14px; font-size: 12px;
  color: #374151; background: none; border: none; cursor: pointer;
  border-bottom: 1px solid #f3f4f6;
  &:last-child { border-bottom: none; }
  &:hover { background: #fef2f2; color: #dc2626; }
`

const Pagination = styled.div`display: flex; align-items: center; justify-content: center; gap: 12px;`

const PageBtn = styled.button`
  padding: 7px 16px; border-radius: 8px; font-size: 13px; font-weight: 500;
  border: 1px solid #e5e7eb; background: #fff; color: #374151; cursor: pointer;
  transition: all .15s;
  &:hover:not(:disabled) { background: #f3f4f6; }
  &:disabled { opacity: .35; cursor: not-allowed; }
`

const PageNum = styled.span`font-size: 13px; color: #9ca3af;`

const Loader = styled.div`
  width: 20px; height: 20px; border: 2px solid #e5e7eb; border-top-color: #5b5ef4;
  border-radius: 50%; animation: ${spin} .7s linear infinite; margin: 60px auto;
`

const EmptyMsg = styled.p`text-align: center; padding: 60px; color: #9ca3af;`
const HintMsg = styled.p`text-align: center; padding: 60px; color: #d1d5db; font-size: 14px;`

// ─── Global search grouped layout ─────────────────────────────────────────────

const GroupsContainer = styled.div`display: flex; flex-direction: column; gap: 28px;`

const GroupSection = styled.div`display: flex; flex-direction: column; gap: 10px;`

const GroupHeader = styled.div<{ $source: string }>`
  display: flex; align-items: center; gap: 8px; padding: 0 2px;
  color: ${({ $source }) => SOURCE_COLORS[$source] ?? '#374151'};
`

const GroupTitle = styled.h3`
  font-size: 13px; font-weight: 700; text-transform: uppercase; letter-spacing: .06em;
`

const GroupCount = styled.span`font-size: 11px; font-weight: 500; color: #9ca3af; margin-left: 2px;`

const CardList = styled.div`display: flex; flex-direction: column; gap: 8px;`

// ─── Bannis inline ────────────────────────────────────────────────────────────

const BannisActiveBanner = styled.div`
  display: flex; align-items: center; justify-content: space-between; padding: 8px 14px;
  background: #fef2f2; border: 1px solid #fca5a5; border-radius: 10px;
  font-size: 12px; color: #dc2626; font-weight: 500;
`

const BannisDeactivateBtn = styled.button`
  padding: 3px 10px; border-radius: 6px; font-size: 11px; font-weight: 600; font-family: inherit;
  border: 1px solid #fca5a5; background: #fff; color: #dc2626; cursor: pointer;
  transition: all .15s;
  &:hover { background: #fee2e2; }
`

const BannisItem = styled.div`
  display: flex; align-items: center; justify-content: space-between;
  background: #f9fafb; border: 1px solid #f3f4f6; border-radius: 10px;
  padding: 10px 14px; gap: 10px;
`

const BannisItemText = styled.div`flex: 1; min-width: 0;`

const BannisItemKind = styled.span`
  font-size: 10px; font-weight: 600; text-transform: uppercase; color: #9ca3af; letter-spacing: .04em;
`

const BannisItemId = styled.p`
  font-size: 12px; color: #374151; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; margin-top: 2px;
`

const UnbanBtn = styled.button`
  padding: 4px 10px; border-radius: 6px; font-size: 11px; font-weight: 600;
  border: 1px solid #10b981; background: #f0fdf4; color: #059669; cursor: pointer; white-space: nowrap;
  transition: all .15s;
  &:hover { background: #dcfce7; }
`

// ─── iMessage ─────────────────────────────────────────────────────────────────

const ImessageLayout = styled.div`
  display: flex; gap: 0; height: 600px; border: 1px solid #e8eaed;
  border-radius: 14px; overflow: hidden; background: #fff;
`
const ContactPanel = styled.div`
  width: 220px; flex-shrink: 0; border-right: 1px solid #e8eaed;
  display: flex; flex-direction: column; background: #f8fafc;
`
const ContactList = styled.div`flex: 1; overflow-y: auto;`
const ContactItem = styled.div<{ $active?: boolean }>`
  padding: 12px 14px; cursor: pointer; border-bottom: 1px solid #f1f3f4;
  background: ${({ $active }) => $active ? '#ede9fe' : 'transparent'};
  &:hover { background: ${({ $active }) => $active ? '#ede9fe' : '#f1f3f4'}; }
`
const ContactPhone = styled.p`
  font-size: 12px; font-weight: 600; color: #1a1d23;
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
`
const ContactPreview = styled.p`
  font-size: 11px; color: #9ca3af; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; margin-top: 2px;
`
const ContactCount = styled.span`font-size: 10px; color: #c4b5fd; font-weight: 500;`
const ConversationPanel = styled.div`flex: 1; display: flex; flex-direction: column; overflow: hidden;`
const ConvHeader = styled.div`
  padding: 12px 16px; border-bottom: 1px solid #e8eaed; font-size: 13px;
  font-weight: 600; color: #1a1d23; background: #fff; flex-shrink: 0;
`
const MessageList = styled.div`
  flex: 1; overflow-y: auto; padding: 16px; display: flex; flex-direction: column; gap: 8px;
`
const MessageRow = styled.div<{ $isMe: boolean }>`
  display: flex; justify-content: ${({ $isMe }) => $isMe ? 'flex-end' : 'flex-start'};
`
const Bubble = styled.div<{ $isMe: boolean }>`
  max-width: 72%; padding: 8px 12px;
  border-radius: ${({ $isMe }) => $isMe ? '16px 16px 4px 16px' : '16px 16px 16px 4px'};
  background: ${({ $isMe }) => $isMe ? '#9333ea' : '#f1f3f4'};
  color: ${({ $isMe }) => $isMe ? '#fff' : '#1a1d23'};
  font-size: 12px; line-height: 1.5; word-break: break-word;
`
const BubbleDate = styled.p<{ $isMe: boolean }>`
  font-size: 10px; color: #9ca3af; margin-top: 2px;
  text-align: ${({ $isMe }) => $isMe ? 'right' : 'left'};
`
const NoConvMsg = styled.div`
  flex: 1; display: flex; align-items: center; justify-content: center; color: #d1d5db; font-size: 13px;
`

const ImsgGroupSection = styled.div`display: flex; flex-direction: column; gap: 8px;`
const ImsgGroupHeader = styled.div`display: flex; align-items: center; gap: 8px; padding: 0 2px; color: #9333ea;`
const ImsgGroupPhone = styled.h3`font-size: 13px; font-weight: 700; text-transform: uppercase; letter-spacing: .06em;`
const ImsgGroupCount = styled.span`font-size: 11px; font-weight: 500; color: #9ca3af;`
const MsgCard = styled.div`background: #fff; border: 1px solid #e8eaed; border-radius: 12px; padding: 12px 16px; box-shadow: 0 1px 3px rgba(0,0,0,.04);`
const MsgMeta = styled.div`display: flex; align-items: center; justify-content: space-between; margin-bottom: 4px;`
const MsgDirection = styled.span<{ $isMe: boolean }>`font-size: 11px; font-weight: 600; color: ${({ $isMe }) => $isMe ? '#9333ea' : '#6b7280'};`
const MsgDate = styled.span`font-size: 11px; color: #9ca3af;`
const MsgText = styled.p`font-size: 12px; color: #374151; line-height: 1.5;`

// ─── File result cards ────────────────────────────────────────────────────────

const FileCard = styled.div`
  background: #fff; border: 1px solid #e8eaed; border-radius: 10px;
  padding: 12px 16px; display: flex; flex-direction: column; gap: 4px;
  cursor: pointer; transition: box-shadow .15s;
  &:hover { box-shadow: 0 2px 8px rgba(0,0,0,.08); }
`
const FileCardName = styled.span`
  font-size: 13px; font-weight: 600; color: #111827;
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
`
const FileCardMeta = styled.span`font-size: 11px; color: #9ca3af;`
const FileCardSnippet = styled.p`
  font-size: 12px; color: #4b5563; margin: 0; line-height: 1.5; overflow: hidden;
  display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical;
`

// ─── FileTypeSelector ─────────────────────────────────────────────────────────

const ExtChipRow = styled.div`display: flex; flex-wrap: wrap; gap: 6px;`
const ExtChip = styled.button<{ $active: boolean }>`
  padding: 4px 10px; border-radius: 8px; font-size: 12px; font-weight: 500; font-family: inherit;
  cursor: pointer; transition: all .15s;
  background: ${({ $active }) => $active ? '#5b5ef4' : '#f3f4f6'};
  color: ${({ $active }) => $active ? '#fff' : '#374151'};
  border: 1px solid ${({ $active }) => $active ? '#5b5ef4' : '#e5e7eb'};
  &:hover { opacity: .85; }
`
const TypeSelectorWrap = styled.div`display: flex; flex-direction: column; gap: 10px; flex: 1;`
const TypeSelectorHeader = styled.button`
  display: flex; align-items: center; gap: 6px; background: none; border: none;
  padding: 0; cursor: pointer; font-family: inherit;
  &:hover span { color: #374151; }
`
const TypeSelectorLabel = styled.span`
  font-size: 11px; font-weight: 600; color: #9ca3af; text-transform: uppercase;
  letter-spacing: .04em; transition: color .15s;
`
const CollapseArrow = styled.span<{ $open: boolean }>`
  font-size: 10px; color: #9ca3af; transition: transform .2s; display: inline-block;
  transform: rotate(${({ $open }) => $open ? '0deg' : '-90deg'});
`
const ActiveBadge = styled.span`
  font-size: 10px; font-weight: 600; background: #5b5ef4; color: #fff; border-radius: 99px; padding: 1px 7px;
`
const FileSearchInput = styled.input`
  width: 100%; box-sizing: border-box; padding: 10px 14px 10px 36px;
  border: 1px solid #e5e7eb; border-radius: 10px; font-size: 14px; font-family: inherit; outline: none;
  background: #fff url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='16' height='16' fill='none' viewBox='0 0 24 24'%3E%3Ccircle cx='11' cy='11' r='7' stroke='%239ca3af' stroke-width='2'/%3E%3Cpath d='M20 20l-3-3' stroke='%239ca3af' stroke-width='2' stroke-linecap='round'/%3E%3C/svg%3E") no-repeat 12px center;
  transition: border-color .15s, box-shadow .15s;
  &:focus { border-color: #5b5ef4; box-shadow: 0 0 0 3px rgba(91,94,244,.1); }
  &::placeholder { color: #9ca3af; }
`

// ─── Helpers ──────────────────────────────────────────────────────────────────

function extractIdentifier(doc: RecentDoc): string | null {
  if (doc.source === 'imessage') { const m = (doc.title ?? '').match(/([+\d]{7,})/); return m ? m[1] : null }
  if (doc.source === 'chrome' || doc.source === 'safari') { try { return new URL(doc.url).hostname } catch { return null } }
  if (doc.source === 'file') { const p = doc.url.split('/'); p.pop(); return p.join('/') || null }
  if (doc.source === 'email') { const m = doc.content?.match(/^De\s*:\s*(.+)/m); return m ? m[1].trim() : null }
  return null
}

function extractSearchIdentifier(source: string, doc: SearchDoc): string | null {
  if (source === 'imessage') { const m = (doc.title ?? '').match(/([+\d]{7,})/); return m ? m[1] : null }
  if (source === 'chrome' || source === 'safari') { try { return new URL(doc.url).hostname } catch { return null } }
  if (source === 'file') { const p = doc.url.split('/'); p.pop(); return p.join('/') || null }
  if (source === 'email') { const m = doc.content?.match(/^De\s*:\s*(.+)/m); return m ? m[1].trim() : null }
  return null
}

function useDebounce<T>(value: T, delay: number): T {
  const [debounced, setDebounced] = useState(value)
  useEffect(() => {
    const t = setTimeout(() => setDebounced(value), delay)
    return () => clearTimeout(t)
  }, [value, delay])
  return debounced
}

// ─── Sub-components ───────────────────────────────────────────────────────────

function FileTypeSelector({ selected, onToggle, searchQ, onSearchChange }: {
  selected: Set<string>; onToggle: (ext: string) => void; searchQ: string; onSearchChange: (q: string) => void
}) {
  const [open, setOpen] = useState(false)
  const { data: preview } = useQuery<Record<string, number>>({
    queryKey: ['index-preview'], queryFn: api.indexPreview, staleTime: 300_000, refetchOnWindowFocus: false,
  })
  const sortedExts = Object.entries(preview ?? {}).sort(([a, ca], [b, cb]) => {
    if (a === 'pdf') return -1; if (b === 'pdf') return 1; return cb - ca
  })
  const placeholder = selected.size === 0
    ? 'Chercher dans tous les fichiers…'
    : `Chercher dans ${[...selected].map(e => `.${e}`).join(', ')}…`
  return (
    <TypeSelectorWrap>
      <TypeSelectorHeader onClick={() => setOpen(o => !o)}>
        <CollapseArrow $open={open}>▼</CollapseArrow>
        <TypeSelectorLabel>Filtrer par type</TypeSelectorLabel>
        {selected.size > 0 && <ActiveBadge>{[...selected].map(e => `.${e}`).join(' ')}</ActiveBadge>}
      </TypeSelectorHeader>
      {open && (
        <ExtChipRow>
          {sortedExts.map(([ext, count]) => (
            <ExtChip key={ext} $active={selected.has(ext)} onClick={() => onToggle(ext)}>
              .{ext} <span style={{ opacity: .6, marginLeft: 4 }}>{count}</span>
            </ExtChip>
          ))}
        </ExtChipRow>
      )}
      <FileSearchInput placeholder={placeholder} value={searchQ} onChange={e => onSearchChange(e.target.value)} />
    </TypeSelectorWrap>
  )
}

function ImessageGroupedResults({ docs, query }: { docs: RecentDoc[]; query: string }) {
  const groups = new Map<string, RecentDoc[]>()
  for (const doc of docs) {
    const m = doc.title?.match(/([+\d]{7,})/); const phone = m ? m[1] : 'Inconnu'
    if (!groups.has(phone)) groups.set(phone, [])
    const arr = groups.get(phone)!; if (arr.length < 5) arr.push(doc)
  }
  if (groups.size === 0) return <EmptyMsg>Aucun résultat.</EmptyMsg>
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 24 }}>
      {Array.from(groups.entries()).map(([phone, msgs]) => (
        <ImsgGroupSection key={phone}>
          <ImsgGroupHeader>
            <ImsgGroupPhone>{phone}</ImsgGroupPhone>
            <ImsgGroupCount>{msgs.length} message{msgs.length > 1 ? 's' : ''}</ImsgGroupCount>
          </ImsgGroupHeader>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
            {msgs.map((doc, i) => {
              const isMe = doc.title?.includes('→') ?? false
              return (
                <MsgCard key={i}>
                  <MsgMeta>
                    <MsgDirection $isMe={isMe}>{isMe ? 'Moi' : phone}</MsgDirection>
                    {doc.source_ts && <MsgDate>{new Date(doc.source_ts * 1000).toLocaleDateString('fr-FR')}</MsgDate>}
                  </MsgMeta>
                  <MsgText>{highlightText(doc.content, query)}</MsgText>
                </MsgCard>
              )
            })}
          </div>
        </ImsgGroupSection>
      ))}
    </div>
  )
}

function ImessageView() {
  const [selectedPhone, setSelectedPhone] = useState<string | null>(null)
  const bottomRef = useRef<HTMLDivElement>(null)
  const { data: contacts, isLoading: loadingContacts } = useQuery<ContactItem[]>({
    queryKey: ['imessage-contacts'], queryFn: () => api.getImessageContacts(), refetchInterval: false,
  })
  const { data: messages, isLoading: loadingMessages } = useQuery<MessageItem[]>({
    queryKey: ['imessage-conv', selectedPhone],
    queryFn: () => api.getImessageConversation(selectedPhone!, 500),
    enabled: !!selectedPhone, refetchInterval: false,
  })
  useEffect(() => { bottomRef.current?.scrollIntoView({ behavior: 'smooth' }) }, [messages])
  return (
    <ImessageLayout>
      <ContactPanel>
        <ContactList>
          {loadingContacts && <Loader style={{ margin: '20px auto' }} />}
          {(contacts ?? []).length === 0 && !loadingContacts && (
            <div style={{ padding: 12, fontSize: 11, color: '#9ca3af', textAlign: 'center' }}>Aucun contact</div>
          )}
          {(contacts ?? []).map(c => (
            <ContactItem key={c.phone} $active={selectedPhone === c.phone} onClick={() => setSelectedPhone(c.phone)}>
              <ContactPhone>{c.phone}</ContactPhone>
              <ContactPreview>{c.last_message}</ContactPreview>
              <ContactCount>{c.count} msg</ContactCount>
            </ContactItem>
          ))}
        </ContactList>
      </ContactPanel>
      <ConversationPanel>
        {selectedPhone
          ? <>
              <ConvHeader>{selectedPhone}</ConvHeader>
              <MessageList>
                {loadingMessages && <Loader style={{ margin: '20px auto' }} />}
                {(messages ?? []).map((m, i) => (
                  <div key={i}>
                    <MessageRow $isMe={m.is_me}><Bubble $isMe={m.is_me}>{m.text}</Bubble></MessageRow>
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

function BanCardItem({ doc, clickable, identifier, query, onBanned }: {
  doc: RecentDoc; clickable: boolean; identifier: string | null; query: string; onBanned: () => void
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
      <Card $clickable={clickable} onClick={clickable ? () => openUrl(doc.source, doc.url) : undefined}>
        <CardTop>
          <Badge $source={doc.source}>{doc.source}</Badge>
          <DocTitle>{highlightText(doc.title || doc.url, query)}</DocTitle>
        </CardTop>
        <DocContent>
          {doc.source === 'email' ? renderEmailContent(doc.content, query) : highlightText(doc.content, query)}
        </DocContent>
      </Card>
      <CardActions>
        <BanBtn onClick={e => { e.stopPropagation(); setMenuOpen(v => !v) }}>⊘ Bannir</BanBtn>
        {menuOpen && (
          <BanMenu>
            <BanMenuItem onClick={() => doBan('url')}>Ce document uniquement</BanMenuItem>
            {identifier && <BanMenuItem onClick={() => doBan('source')}>Tout de : {identifier}</BanMenuItem>}
          </BanMenu>
        )}
      </CardActions>
    </CardWrapper>
  )
}

function GlobalResultCard({ doc, source, query, onBanned }: {
  doc: SearchDoc; source: string; query: string; onBanned: () => void
}) {
  const [menuOpen, setMenuOpen] = useState(false)
  const clickable = CLICKABLE_SOURCES.has(source)
  const identifier = extractSearchIdentifier(source, doc)
  async function doBan(kind: 'url' | 'source') {
    setMenuOpen(false)
    if (kind === 'url') await api.banUrl(doc.url)
    else if (identifier) await api.banSourceItem(source, identifier)
    onBanned()
  }
  return (
    <CardWrapper>
      <Card $clickable={clickable} onClick={clickable ? () => openUrl(source, doc.url) : undefined}>
        <CardHeader>
          <DocTitle>{highlightText(doc.title || doc.url, query)}</DocTitle>
          {doc.date && <CardDate>{doc.date}</CardDate>}
        </CardHeader>
        <DocContent>
          {source === 'email' ? renderEmailContent(doc.content, query) : highlightText(doc.content, query)}
        </DocContent>
        <CardUrl>{doc.url}</CardUrl>
      </Card>
      <CardActions>
        <BanBtn onClick={e => { e.stopPropagation(); setMenuOpen(v => !v) }}>⊘ Bannir</BanBtn>
        {menuOpen && (
          <BanMenu>
            <BanMenuItem onClick={() => doBan('url')}>Ce document uniquement</BanMenuItem>
            {identifier && <BanMenuItem onClick={() => doBan('source')}>Tout de : {identifier}</BanMenuItem>}
          </BanMenu>
        )}
      </CardActions>
    </CardWrapper>
  )
}


function BannisInline({ source }: { source: string }) {
  const queryClient = useQueryClient()
  const { data, isLoading, refetch } = useQuery({ queryKey: ['blacklist'], queryFn: api.getBlacklist })
  const filtered = (data?.entries ?? []).filter(e => source === 'all' ? true : e.source === source)
  async function doUnban(entry: BlacklistEntry) {
    if (entry.kind === 'url') await api.unbanUrl(entry.identifier)
    else await api.unbanSourceItem(entry.source, entry.identifier)
    await refetch()
    queryClient.invalidateQueries({ queryKey: ['recent'] })
    queryClient.invalidateQueries({ queryKey: ['search'] })
  }
  if (isLoading) return <Loader />
  if (filtered.length === 0) return <EmptyMsg>Aucun élément banni{source !== 'all' ? ' dans cette source' : ''}.</EmptyMsg>
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
      {filtered.map((entry, i) => (
        <BannisItem key={i}>
          <BannisItemText>
            <BannisItemKind>
              {entry.kind === 'url'
                ? `Document banni (${SOURCE_LABELS[entry.source] ?? entry.source})`
                : `Tout de (${SOURCE_LABELS[entry.source] ?? entry.source})`
              }
            </BannisItemKind>
            <BannisItemId title={entry.identifier}>{entry.title || entry.identifier}</BannisItemId>
          </BannisItemText>
          <UnbanBtn onClick={() => doUnban(entry)}>Débannir</UnbanBtn>
        </BannisItem>
      ))}
    </div>
  )
}

// ─── Page principale ──────────────────────────────────────────────────────────

export default function RecentPage() {
  const [activeTab, setActiveTab] = useState<string>('global')
  const [searchQ, setSearchQ]     = useState('')
  const debouncedQ                = useDebounce(searchQ, 350)
  const [filterFrom, setFilterFrom] = useState('')
  const [filterTo,   setFilterTo]   = useState('')
  const [menuOpen,   setMenuOpen]   = useState(false)
  const [showBannis, setShowBannis] = useState(false)
  const [fileTypes,  setFileTypes]  = useState<Set<string>>(new Set())
  const [page, setPage]             = useState(0)
  const menuRef    = useRef<HTMLDivElement>(null)
  const queryClient = useQueryClient()
  const limit = 20

  // Close dots menu on outside click
  useEffect(() => {
    if (!menuOpen) return
    function handle(e: MouseEvent) {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) setMenuOpen(false)
    }
    document.addEventListener('mousedown', handle)
    return () => document.removeEventListener('mousedown', handle)
  }, [menuOpen])

  const [showBlacklistPanel, setShowBlacklistPanel] = useState(false)
  const { data: blacklistData } = useQuery({ queryKey: ['blacklist'], queryFn: api.getBlacklist })
  const bannedCount = blacklistData?.entries.length ?? 0

  const { data: statusData } = useQuery<StatusData>({
    queryKey: ['status'], queryFn: api.getStatus, refetchInterval: false,
  })
  const activeSources = SOURCES.filter(s => s in (statusData?.sources ?? {}))

  function selectTab(tab: string) {
    setActiveTab(tab); setPage(0); setSearchQ('')
    setFilterFrom(''); setFilterTo(''); setShowBannis(false); setMenuOpen(false)
  }

  const isGlobal   = activeTab === 'global'
  const isFile     = activeTab === 'file'
  const isImessage = activeTab === 'imessage'
  const hasFilters = !!debouncedQ || !!filterFrom || !!filterTo
  const isImessageNoFilter = isImessage && !hasFilters
  const isFileSearch = isFile && debouncedQ.trim().length >= 3
  const hasDateFilters = !!filterFrom || !!filterTo

  // Global search
  const { data: searchData, isLoading: searchLoading } = useQuery<GroupedSearchResponse>({
    queryKey: ['search', debouncedQ, filterFrom, filterTo],
    queryFn:  () => api.search(debouncedQ, { from: filterFrom || undefined, to: filterTo || undefined }),
    enabled:  isGlobal && debouncedQ.length > 1 && !showBannis,
    refetchInterval: false,
  })

  // Per-source recent
  const { data: recentData, isLoading: recentLoading } = useQuery<RecentDoc[]>({
    queryKey: ['recent', activeTab, page, debouncedQ, filterFrom, filterTo],
    queryFn:  () => api.getRecent(activeTab, limit, page * limit, {
      q: debouncedQ || undefined, from: filterFrom || undefined, to: filterTo || undefined,
    }),
    enabled: !isGlobal && !isImessageNoFilter && !isFile && activeSources.length > 0 && !showBannis,
    refetchInterval: false,
  })

  // File live search
  const { data: fileResults, isFetching: fileLoading } = useQuery({
    queryKey: ['files-live-search', debouncedQ, [...fileTypes].sort().join(',')],
    queryFn:  () => api.filesSearch(debouncedQ, [...fileTypes].join(',')),
    enabled:  isFileSearch && !showBannis,
    staleTime: 120_000, refetchOnWindowFocus: false, refetchOnMount: false,
  })

  return (
    <Page>
      <PageTitle>Documents récents</PageTitle>

      {/* Carte liste noire */}
      <BlacklistCard>
        <span style={{ fontSize: 13, color: '#6b7280' }}>Éléments exclus des résultats MCP</span>
        <div style={{ display: 'flex', alignItems: 'center' }}>
          <BlacklistCount $n={bannedCount}>{bannedCount} banni{bannedCount !== 1 ? 's' : ''}</BlacklistCount>
          <ManageBtn onClick={() => setShowBlacklistPanel(true)}>Gérer</ManageBtn>
        </div>
      </BlacklistCard>
      {showBlacklistPanel && <BlacklistPanel source="all" onClose={() => setShowBlacklistPanel(false)} />}

      {/* Tabs */}
      <TabRow>
        <Tab $active={isGlobal} onClick={() => selectTab('global')}>🔍 Global</Tab>
        {activeSources.map(s => (
          <Tab key={s} $active={activeTab === s} onClick={() => selectTab(s)}>
            {SOURCE_LABELS[s]}
          </Tab>
        ))}
      </TabRow>

      {/* Toolbar */}
      <ToolRow>
        {isFile ? (
          <FileTypeSelector
            selected={fileTypes}
            onToggle={ext => setFileTypes(prev => { const n = new Set(prev); n.has(ext) ? n.delete(ext) : n.add(ext); return n })}
            searchQ={searchQ}
            onSearchChange={q => { setSearchQ(q); setPage(0) }}
          />
        ) : (
          <SearchInputWrap>
            <icons.Search size={16} />
            <SearchInputField
              type="text"
              value={searchQ}
              onChange={e => { setSearchQ(e.target.value); setPage(0) }}
              placeholder={isGlobal ? 'Chercher dans toute ta mémoire…' : `Chercher dans ${SOURCE_LABELS[activeTab] ?? activeTab}…`}
              autoFocus={isGlobal}
            />
            {searchQ && <ClearSearchBtn onClick={() => setSearchQ('')}>✕</ClearSearchBtn>}
          </SearchInputWrap>
        )}

        <DotsMenuWrap ref={menuRef}>
          
          <DotsBtn $active={menuOpen || showBannis || hasDateFilters} onClick={() => setMenuOpen(v => !v)}>
            ···
          </DotsBtn>
          {menuOpen && (
            <DotsDropdown>
              <DateMenuRow>
                <DateMenuLabel>Du</DateMenuLabel>
                <DateMenuInput type="date" value={filterFrom} onChange={e => { setFilterFrom(e.target.value); setPage(0) }} />
                <br /><DateMenuLabel>Au</DateMenuLabel>
                <DateMenuInput type="date" value={filterTo} onChange={e => { setFilterTo(e.target.value); setPage(0) }} />
                {hasDateFilters && <ClearDateBtn onClick={() => { setFilterFrom(''); setFilterTo('') }}>✕</ClearDateBtn>}
              </DateMenuRow>
              <MenuSep />
              <BannisToggle $active={showBannis} onClick={() => { setShowBannis(v => !v); setMenuOpen(false) }}>
                ⊘ Bannis {showBannis ? '(actif)' : ''}
              </BannisToggle>
            </DotsDropdown>
          )}
        </DotsMenuWrap>
      </ToolRow>

      {/* Bannis mode (inline) */}  
      {showBannis && (
        <>
          <BannisActiveBanner>
            <span>⊘ Mode bannis actif</span>
            <BannisDeactivateBtn onClick={() => setShowBannis(false)}>Désactiver</BannisDeactivateBtn>
          </BannisActiveBanner>
          <BannisInline source={isGlobal ? 'all' : activeTab} />
        </>
      )}

      {/* Fichiers */}
      {!showBannis && isFile && (
        <>
          {isFileSearch ? (
            <>
              {fileLoading && <Loader />}
              {!fileLoading && fileResults?.length === 0 && <EmptyMsg>Aucun fichier trouvé pour « {searchQ} »</EmptyMsg>}
              <DocList>
                {fileResults?.map((f, i) => (
                  <FileCard key={i} onClick={() => window.open(`file://${f.path}`, '_blank')}>
                    <FileCardName>{f.name}</FileCardName>
                    <FileCardMeta>.{f.ext} · {f.size_kb > 0 ? `${f.size_kb} KB` : '< 1 KB'} · {f.path}</FileCardMeta>
                    {f.snippet && <FileCardSnippet>{highlightText(f.snippet, debouncedQ)}</FileCardSnippet>}
                  </FileCard>
                ))}
              </DocList>
            </>
          ) : (
            <EmptyMsg>Tape 3 caractères minimum pour chercher dans tes fichiers</EmptyMsg>
          )}
        </>
      )}

      {/* Global */}
      {!showBannis && isGlobal && (
        <>
          {searchLoading && <Loader />}
          {!searchLoading && debouncedQ.length > 1 && !(searchData?.groups.length) && (
            <EmptyMsg>Aucun résultat pour « {debouncedQ} »</EmptyMsg>
          )}
          {!debouncedQ && <HintMsg>Tape un mot-clé pour chercher dans toute ta mémoire…</HintMsg>}
          {!!searchData?.groups.length && (
            <GroupsContainer>
              {searchData.groups.map(group => (
                <GroupSection key={group.source}>
                  <GroupHeader $source={group.source}>
                    <GroupTitle>{SOURCE_LABELS[group.source] ?? group.source}</GroupTitle>
                    <GroupCount>{group.results.length} résultat{group.results.length > 1 ? 's' : ''}</GroupCount>
                  </GroupHeader>
                  <CardList>
                    {group.results.map((doc, i) => (
                      <GlobalResultCard
                        key={i} doc={doc} source={group.source} query={debouncedQ}
                        onBanned={() => queryClient.invalidateQueries({ queryKey: ['search', debouncedQ] })}
                      />
                    ))}
                  </CardList>
                </GroupSection>
              ))}
            </GroupsContainer>
          )}
        </>
      )}

      {/* iMessage sans filtre → vue 2 colonnes */}
      {!showBannis && isImessageNoFilter && <ImessageView />}

      {/* iMessage avec filtre → résultats groupés */}
      {!showBannis && isImessage && hasFilters && (
        <>
          {recentLoading && <Loader />}
          {!recentLoading && recentData && <ImessageGroupedResults docs={recentData} query={debouncedQ} />}
          {recentData && recentData.length > 0 && (
            <Pagination>
              <PageBtn onClick={() => setPage(p => p - 1)} disabled={page === 0}>← Précédent</PageBtn>
              <PageNum>Page {page + 1}</PageNum>
              <PageBtn onClick={() => setPage(p => p + 1)} disabled={recentData.length < limit}>Suivant →</PageBtn>
            </Pagination>
          )}
        </>
      )}

      {/* Autres sources */}
      {!showBannis && !isGlobal && !isFile && !isImessage && (
        <>
          {recentLoading && <Loader />}
          {!recentLoading && recentData?.length === 0 && (
            <EmptyMsg>Aucun document{hasFilters ? ' correspondant' : ' dans cette source'}.</EmptyMsg>
          )}
          <DocList>
            {recentData?.map((doc, i) => (
              <BanCardItem
                key={i} doc={doc} clickable={CLICKABLE_SOURCES.has(doc.source)}
                identifier={extractIdentifier(doc)} query={debouncedQ}
                onBanned={() => queryClient.invalidateQueries({ queryKey: ['recent', activeTab, page] })}
              />
            ))}
          </DocList>
          {recentData && recentData.length > 0 && (
            <Pagination>
              <PageBtn onClick={() => setPage(p => p - 1)} disabled={page === 0}>← Précédent</PageBtn>
              <PageNum>Page {page + 1}</PageNum>
              <PageBtn onClick={() => setPage(p => p + 1)} disabled={recentData.length < limit}>Suivant →</PageBtn>
            </Pagination>
          )}
        </>
      )}
    </Page>
  )
}
