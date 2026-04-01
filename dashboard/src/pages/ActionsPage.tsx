import { useState, useEffect, useRef } from 'react'
import { Zap, PlugZap, Shield, Database, RefreshCw, Key, UserRound, ShieldOff } from 'lucide-react'
import styled, { keyframes } from 'styled-components'
import { useQuery, useQueryClient, useMutation } from '@tanstack/react-query'
import { api } from '../api'
import type { ActionRequest, ActionEvent, DbSecurityConfig, ColumnRule } from '../api'
import { PrivacyPanel } from '../components/PrivacyPanel'

// ─── Styles ───────────────────────────────────────────────────────────────────

const spin = keyframes`to { transform: rotate(360deg); }`

const Layout = styled.div`display: flex; flex-direction: column; gap: 0; min-height: 0;`

const TopTabBar = styled.div`
  display: flex; align-items: center; gap: 2px;
  border-bottom: 1px solid #e8eaed;
  margin-bottom: 24px;
  padding-bottom: 0;
`

const TopTabItem = styled.button<{ $active: boolean }>`
  display: flex; align-items: center; gap: 7px;
  padding: 8px 14px 10px; border: none; cursor: pointer; background: transparent;
  font-size: 13px; font-weight: ${({ $active }) => $active ? '600' : '500'};
  color: ${({ $active }) => $active ? '#1a1d23' : '#6b7280'};
  border-bottom: 2px solid ${({ $active }) => $active ? '#5b5ef4' : 'transparent'};
  margin-bottom: -1px;
  transition: color .15s, border-color .15s;
  &:hover { color: #1a1d23; }
`

const SseStatusInline = styled.span`
  margin-left: auto; display: flex; align-items: center; gap: 6px;
  font-size: 11px; color: #9ca3af; padding-bottom: 10px;
`

const Content = styled.div`min-width: 0;`

const SideNavItem = styled.button<{ $active: boolean }>`
  display: flex; align-items: center; gap: 8px;
  width: 100%; padding: 7px 10px; border-radius: 7px;
  border: none; cursor: pointer; text-align: left;
  font-size: 12px; font-weight: ${({ $active }) => $active ? '600' : '500'};
  background: ${({ $active }) => $active ? '#ededff' : 'transparent'};
  color: ${({ $active }) => $active ? '#5b5ef4' : '#6b7280'};
  transition: all .15s;
  &:hover { background: ${({ $active }) => $active ? '#ededff' : '#f3f4f6'}; }
`

const ContentHeader = styled.div`
  display: flex; align-items: center; justify-content: space-between;
  margin-bottom: 20px;
`

const PageTitle = styled.h1`font-size: 18px; font-weight: 700; color: #1a1d23; letter-spacing: -.02em;`

// ── Autorisations MCP ────────────────────────────────────────────────────────

const PermSection = styled.div`
  background: #fff; border: 1px solid #e8eaed; border-radius: 14px;
  overflow: hidden; box-shadow: 0 1px 3px rgba(0,0,0,.05);
`

const PermHeader = styled.div`
  padding: 18px 20px 14px; border-bottom: 1px solid #f3f4f6;
`

const PermTitle = styled.h2`font-size: 14px; font-weight: 600; color: #1a1d23; margin: 0;`

const PermDesc = styled.p`font-size: 12px; color: #6b7280; margin: 4px 0 0; line-height: 1.5;`


const PermLabel = styled.span`font-size: 13px; font-weight: 500; color: #1a1d23;`

const PermHint = styled.span`font-size: 11px; color: #9ca3af; display: block; margin-top: 2px;`

const Toggle = styled.button<{ $on: boolean }>`
  width: 40px; height: 22px; border-radius: 11px; border: none; cursor: pointer;
  background: ${({ $on }) => $on ? '#5b5ef4' : '#d1d5db'};
  position: relative; transition: background .2s; flex-shrink: 0;
  &::after {
    content: ''; position: absolute; width: 16px; height: 16px;
    border-radius: 50%; background: white; top: 3px;
    left: ${({ $on }) => $on ? '21px' : '3px'}; transition: left .2s;
    box-shadow: 0 1px 3px rgba(0,0,0,.2);
  }
`

// ── Tableau sources unifié ───────────────────────────────────────────────────

const SourceTable = styled.table`width: 100%; border-collapse: collapse;`

const SourceTh = styled.th<{ $center?: boolean }>`
  text-align: ${({ $center }) => $center ? 'center' : 'left'};
  font-size: 11px; font-weight: 600; color: #9ca3af;
  text-transform: uppercase; letter-spacing: .05em;
  padding: 0 20px 12px; border-bottom: 1px solid #f3f4f6;
`

const SourceTd = styled.td<{ $center?: boolean }>`
  padding: 12px 20px; border-bottom: 1px solid #f9fafb;
  text-align: ${({ $center }) => $center ? 'center' : 'left'};
  vertical-align: middle;
  &:last-child { border-bottom: none; }
`


// ── Alias Engine ─────────────────────────────────────────────────────────────

const AliasTable = styled.table`width: 100%; border-collapse: collapse;`
const AliasTh = styled.th`
  text-align: left; font-size: 11px; font-weight: 600; color: #9ca3af;
  text-transform: uppercase; letter-spacing: .05em;
  padding: 0 12px 10px; border-bottom: 1px solid #f3f4f6;
`
const AliasTd = styled.td`
  padding: 9px 12px; font-size: 13px; color: #1a1d23;
  border-bottom: 1px solid #f9fafb; vertical-align: middle;
`
const AliasArrow = styled(AliasTd)`color: #d1d5db; font-size: 16px; width: 32px; text-align: center;`
const AliasMuted = styled(AliasTd)`color: #6b7280;`
const AliasDelBtn = styled.button`
  background: none; border: 1px solid #fca5a5; color: #ef4444;
  border-radius: 6px; padding: 3px 10px; font-size: 12px; cursor: pointer;
  &:hover { background: #fef2f2; }
`
const AliasAddRow = styled.div`display: flex; gap: 8px; margin-top: 14px; align-items: center;`
const AliasInput = styled.input`
  flex: 1; border: 1px solid #e8eaed; border-radius: 8px;
  padding: 7px 11px; font-size: 13px; color: #1a1d23; outline: none;
  &:focus { border-color: #5b5ef4; box-shadow: 0 0 0 3px rgba(91,94,244,.08); }
  &::placeholder { color: #9ca3af; }
`
const AliasAddBtn = styled.button`
  background: #5b5ef4; color: #fff; border: none; border-radius: 8px;
  padding: 7px 16px; font-size: 13px; font-weight: 600; cursor: pointer; white-space: nowrap;
  &:hover { opacity: .88; } &:disabled { opacity: .4; cursor: default; }
`

// ── Journal ──────────────────────────────────────────────────────────────────

// Couleurs de marque par connecteur
const CONNECTOR_COLORS: Record<string, string> = {
  github:     '#24292e',
  gitlab:     '#e24329',
  linear:     '#5e6ad2',
  jira:       '#0052cc',
  sentry:     '#362d59',
  vercel:     '#000000',
  railway:    '#c10000',
  render:     '#46e3b7',
  stripe:     '#635bff',
  hubspot:    '#ff7a59',
  discord:    '#5865f2',
  figma:      '#f24e1e',
  notion:     '#000000',
  slack:      '#4a154b',
  supabase:   '#3ecf8e',
  cloudflare: '#f38020',
  reddit:     '#ff4500',
  calendly:   '#006bff',
  posthog:    '#f54e00',
  resend:     '#000000',
  twilio:     '#f22f46',
  google:     '#4285f4',
  gcal:       '#4285f4',
  gmail:      '#ea4335',
  search:     '#6b7280',
  find:       '#6b7280',
  fetch:      '#6b7280',
  get:        '#6b7280',
  list:       '#6b7280',
  index:      '#6b7280',
  osmozzz:    '#5b5ef4',
  act:        '#f59e0b',
}

function connectorColor(tool: string): string {
  // Format MCP proxy : "github:list_commits"
  if (tool.includes(':')) {
    const prefix = tool.split(':')[0].toLowerCase()
    return CONNECTOR_COLORS[prefix] ?? '#5b5ef4'
  }
  // Format natif : "linear_list_issues", "vercel_list_deployments"
  const prefix = tool.split('_')[0].toLowerCase()
  return CONNECTOR_COLORS[prefix] ?? '#5b5ef4'
}

function parseToolDisplay(tool: string): { connector: string; action: string } {
  if (tool.includes(':')) {
    const [connector, ...rest] = tool.split(':')
    return { connector, action: rest.join(':').replace(/_/g, ' ') }
  }
  const parts = tool.split('_')
  const connector = parts[0]
  const action = parts.slice(1).join(' ')
  return { connector, action: action || connector }
}

const JournalList = styled.div`display: flex; flex-direction: column;`

const JournalRow = styled.div<{ $blocked: boolean; $color: string }>`
  display: flex; flex-direction: column; gap: 3px;
  padding: 8px 10px 8px 14px;
  border-bottom: 1px solid #f3f4f6;
  border-left: 3px solid ${({ $blocked, $color }) => $blocked ? '#dc2626' : $color};
  background: ${({ $blocked }) => $blocked ? '#fff5f5' : 'transparent'};
  transition: background 0.1s;
  &:hover { background: ${({ $blocked }) => $blocked ? '#fff0f0' : '#fafafa'}; }
  &:last-child { border-bottom: none; }
`

const JournalMeta = styled.div`display: flex; align-items: center; gap: 6px; flex-wrap: wrap;`

const JournalTime = styled.span`font-size: 11px; color: #b0b7c3; white-space: nowrap; font-variant-numeric: tabular-nums;`

const JournalConnector = styled.span<{ $color: string }>`
  font-size: 11px; font-weight: 700;
  color: ${({ $color }) => $color};
  white-space: nowrap;
  letter-spacing: 0.01em;
`

const JournalAction = styled.span`
  font-size: 11px; font-weight: 400; color: #9ca3af; white-space: nowrap;
`

const JournalResultsBadge = styled.span<{ $blocked: boolean }>`
  font-size: 10px; font-weight: 600; padding: 1px 6px; border-radius: 10px;
  background: ${({ $blocked }) => $blocked ? '#fee2e2' : '#f0f0ff'};
  color: ${({ $blocked }) => $blocked ? '#dc2626' : '#6366f1'};
  white-space: nowrap;
  margin-left: auto;
`

const JournalQuery = styled.span`
  font-size: 12px; color: #4b5563; line-height: 1.5;
  word-break: break-word;
  padding: 2px 6px; border-radius: 4px;
  background: #f9fafb;
  display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical;
  overflow: hidden;
  font-style: italic;
`



const DataBox = styled.div`
  margin: 6px 0 0; padding: 10px 14px; border-radius: 8px;
  background: #f8fafc; border: 1px solid #e8eaed;
  font-size: 11px; color: #374151; line-height: 1.5;
  white-space: pre-wrap; word-break: break-word;
  max-height: 220px; overflow-y: auto; font-family: 'SF Mono', monospace;
`

const SecurityBox = styled.div`
  margin: 6px 0 0; border-radius: 8px;
  background: #fffbeb; border: 1px solid #fde68a;
  overflow: hidden;
`

const SecurityBoxHeader = styled.div`
  display: flex; align-items: center; gap: 6px;
  padding: 6px 12px; border-bottom: 1px solid #fde68a;
  font-size: 10px; font-weight: 700; text-transform: uppercase;
  letter-spacing: .06em; color: #92400e;
`

const SecurityGroup = styled.div`
  padding: 8px 12px 6px;
  border-bottom: 1px solid #fef3c7;
  &:last-child { border-bottom: none; }
`

const SecurityGroupTitle = styled.div<{ $color: string }>`
  display: flex; align-items: center; gap: 5px;
  font-size: 10px; font-weight: 700; text-transform: uppercase;
  letter-spacing: .05em; color: ${({ $color }) => $color};
  margin-bottom: 5px;
`

const SecurityMaskList = styled.div`
  display: flex; flex-direction: column; gap: 2px;
`

const SecurityMaskItem = styled.div`
  display: flex; align-items: baseline; gap: 6px;
  font-size: 11px; font-family: 'SF Mono', monospace;
`

const SecurityMaskNum = styled.span`
  color: #9ca3af; font-size: 10px; font-weight: 700; flex-shrink: 0; min-width: 22px;
`

const SecurityMaskEmail = styled.span`color: #1a1d23;`

const SecurityRow = styled.div`
  display: flex; align-items: center; gap: 10px;
  padding: 3px 0;
`

const SecurityRealValue = styled.span`
  font-size: 11px; font-family: 'SF Mono', monospace; color: #1a1d23; flex-shrink: 0;
`

const SecurityArrow = styled.span`font-size: 11px; color: #9ca3af; flex-shrink: 0;`

const SecurityReplaced = styled.span<{ $kind: 'block' | 'tokenize' | 'alias' | 'mask' }>`
  font-size: 11px; font-family: 'SF Mono', monospace; font-weight: 600;
  color: ${({ $kind }) =>
    $kind === 'block' || $kind === 'mask' ? '#dc2626' :
    $kind === 'alias'    ? '#059669' :
    '#d97706'};
`

const SecurityBadge = styled.span<{ $kind: 'block' | 'tokenize' | 'alias' | 'mask' }>`
  font-size: 10px; font-weight: 700; padding: 1px 6px; border-radius: 4px; margin-left: 4px;
  background: ${({ $kind }) =>
    $kind === 'block' || $kind === 'mask' ? '#fee2e2' :
    $kind === 'alias'    ? '#d1fae5' :
    '#fef3c7'};
  color: ${({ $kind }) =>
    $kind === 'block' || $kind === 'mask' ? '#dc2626' :
    $kind === 'alias'    ? '#059669' :
    '#d97706'};
`

// ── Actions (tabs + cards) ───────────────────────────────────────────────────

const ActionsBlock = styled.div`display: flex; flex-direction: column; gap: 12px;`


const BadgeCount = styled.span`
  display: inline-flex; align-items: center; justify-content: center;
  min-width: 18px; height: 18px; padding: 0 5px;
  background: #ef4444; color: #fff; border-radius: 99px;
  font-size: 10px; font-weight: 700; margin-left: 6px;
`

const EmptyMsg = styled.p`text-align: center; padding: 60px; color: #9ca3af;`

const Loader = styled.div`
  width: 20px; height: 20px; border: 2px solid #e5e7eb; border-top-color: #5b5ef4;
  border-radius: 50%; animation: ${spin} .7s linear infinite; margin: 60px auto;
`

const CardList = styled.div`display: flex; flex-direction: column; gap: 12px;`

const ActionCard = styled.div<{ $status: string }>`
  background: #fff;
  border: 1px solid #e5e7eb;
  border-radius: 12px; padding: 16px 18px;
  box-shadow: 0 1px 4px rgba(0,0,0,.04);
  position: relative;
  transition: box-shadow .15s;
  &:hover { box-shadow: 0 2px 8px rgba(0,0,0,.07); }
`

const CardTop = styled.div`
  display: flex; align-items: center;
  margin-bottom: 10px; gap: 8px; padding-right: 70px;
`

const CardTopLeft = styled.div`display: flex; align-items: center; gap: 8px; min-width: 0; flex: 1;`

const SourceBadge = styled.span`
  font-size: 11px; font-weight: 700; letter-spacing: .04em;
  padding: 3px 8px; border-radius: 5px; background: #ededff; color: #5b5ef4;
  white-space: nowrap; flex-shrink: 0;
`

const ToolName = styled.span`
  font-size: 13px; font-weight: 600; color: #1a1d23;
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
`

const StatusPill = styled.span<{ $status: string }>`
  position: absolute; top: 12px; right: 14px;
  font-size: 10px; font-weight: 600; padding: 2px 8px; border-radius: 99px;
  white-space: nowrap;
  background: ${({ $status }) =>
    $status === 'pending'  ? '#fef3c7' :
    $status === 'approved' ? '#d1fae5' :
    $status === 'rejected' ? '#fee2e2' :
    '#f3f4f6'};
  color: ${({ $status }) =>
    $status === 'pending'  ? '#92400e' :
    $status === 'approved' ? '#065f46' :
    $status === 'rejected' ? '#991b1b' :
    '#6b7280'};
`

const StatusLabel: Record<string, string> = {
  pending:  'En attente',
  approved: 'Approuvée',
  rejected: 'Refusée',
  expired:  'Expirée',
}

const ParamsGrid = styled.div`
  background: #f8fafc; border: 1px solid #e8eaed; border-radius: 8px;
  padding: 10px 14px; display: flex; flex-direction: column; gap: 4px;
`

const ParamRow = styled.div`display: flex; gap: 8px; align-items: baseline; min-width: 0;`

const ParamKey = styled.span`
  font-size: 11px; font-weight: 600; color: #9ca3af; white-space: nowrap;
  text-transform: lowercase; font-family: 'SF Mono', monospace; flex-shrink: 0;
`

const ParamVal = styled.span`
  font-size: 12px; color: #374151; word-break: break-word; line-height: 1.5;
`

const PreviewBox = styled.pre`
  background: #f8fafc; border: 1px solid #e8eaed; border-radius: 8px;
  padding: 10px 14px; font-size: 12px; color: #374151; line-height: 1.6;
  white-space: pre-wrap; word-break: break-word; margin: 0; font-family: inherit;
`

const CardFooter = styled.div`
  display: flex; align-items: center; justify-content: space-between;
  margin-top: 10px;
`

const CardDate = styled.span`font-size: 11px; color: #9ca3af;`

const TimerBadge = styled.span<{ $urgent: boolean }>`
  font-size: 11px; font-weight: 600;
  color: ${({ $urgent }) => $urgent ? '#dc2626' : '#9ca3af'};
`

const ActionBtns = styled.div`display: flex; gap: 8px; margin-top: 12px;`

const ApproveBtn = styled.button`
  flex: 1; padding: 9px 14px; border-radius: 8px; font-size: 13px; font-weight: 600;
  border: none; background: #10b981; color: #fff; cursor: pointer; transition: background .15s;
  display: flex; align-items: center; justify-content: center; gap: 5px;
  &:hover { background: #059669; }
  &:disabled { opacity: .5; cursor: not-allowed; }
`

const RejectBtn = styled.button`
  padding: 9px 18px; border-radius: 8px; font-size: 13px; font-weight: 600;
  border: 1px solid #e5e7eb; background: #fff; color: #6b7280; cursor: pointer; transition: all .15s;
  display: flex; align-items: center; gap: 5px;
  &:hover { border-color: #fca5a5; color: #dc2626; background: #fef2f2; }
  &:disabled { opacity: .5; cursor: not-allowed; }
`

const LiveDot = styled.span<{ $active: boolean }>`
  display: inline-block; width: 7px; height: 7px; border-radius: 50%;
  background: ${({ $active }) => $active ? '#10b981' : '#d1d5db'};
  margin-right: 6px;
`


const ExecResult = styled.div<{ $ok: boolean }>`
  margin-top: 12px; padding: 10px 14px; border-radius: 8px; font-size: 12px; font-weight: 500;
  background: ${({ $ok }) => $ok ? '#d1fae5' : '#fee2e2'};
  color: ${({ $ok }) => $ok ? '#065f46' : '#991b1b'};
`

// ── DB Security ──────────────────────────────────────────────────────────────

const DbTableCard = styled.div`
  background: #fff; border: 1px solid #e8eaed; border-radius: 14px;
  overflow: hidden; box-shadow: 0 1px 3px rgba(0,0,0,.05); margin-bottom: 12px;
`

const DbTableHeader = styled.div`
  display: flex; align-items: center; justify-content: space-between;
  padding: 14px 18px; border-bottom: 1px solid #f3f4f6;
  background: #fafafa;
`

const DbTableName = styled.span`font-size: 13px; font-weight: 700; color: #1a1d23; font-family: 'SF Mono', monospace;`

const DbColRow = styled.div`
  display: flex; align-items: center; padding: 10px 18px;
  border-bottom: 1px solid #f9fafb;
  &:last-child { border-bottom: none; }
`

const DbColName = styled.span`font-size: 13px; font-weight: 500; color: #374151; flex: 1; font-family: 'SF Mono', monospace; font-size: 12px;`

const DbColType = styled.span`font-size: 11px; color: #9ca3af; margin-right: 16px; min-width: 80px;`

const RuleSelector = styled.div`display: flex; gap: 4px;`

const RuleBtn = styled.button<{ $active: boolean; $variant: 'free' | 'tokenize' | 'block' }>`
  padding: 4px 10px; border-radius: 6px; font-size: 11px; font-weight: 600;
  border: 1px solid ${({ $active, $variant }) =>
    !$active ? '#e5e7eb' :
    $variant === 'free'     ? '#10b981' :
    $variant === 'tokenize' ? '#f59e0b' :
    '#ef4444'};
  background: ${({ $active, $variant }) =>
    !$active ? '#fff' :
    $variant === 'free'     ? '#d1fae5' :
    $variant === 'tokenize' ? '#fef3c7' :
    '#fee2e2'};
  color: ${({ $active, $variant }) =>
    !$active ? '#9ca3af' :
    $variant === 'free'     ? '#065f46' :
    $variant === 'tokenize' ? '#92400e' :
    '#991b1b'};
  cursor: pointer; transition: all .12s;
  &:hover { opacity: .8; }
`

const DbToolbar = styled.div`display: flex; align-items: center; gap: 10px; margin-bottom: 20px;`

const DbProjectBadge = styled.div`
  display: flex; align-items: center; gap: 6px;
  padding: 6px 12px; border-radius: 8px; font-size: 12px; font-weight: 600;
  background: #ededff; color: #5b5ef4; border: 1px solid #d8d8ff;
`

const DbDeleteBtn = styled.button`
  padding: 7px 14px; border-radius: 8px; font-size: 13px; font-weight: 500;
  border: 1px solid #fecaca; background: #fff5f5; color: #ef4444; cursor: pointer;
  transition: all .15s;
  &:hover { background: #fee2e2; }
`

const ProjectSelect = styled.select`
  padding: 7px 12px; border: 1px solid #e8eaed; border-radius: 9px;
  font-size: 13px; color: #1a1d23; background: #fff; outline: none; cursor: pointer;
  &:focus { border-color: #5b5ef4; box-shadow: 0 0 0 3px rgba(91,94,244,.08); }
  &:disabled { opacity: .5; cursor: not-allowed; }
`

const DbEmptyMsg = styled.p`
  text-align: center; padding: 48px 24px; color: #9ca3af; font-size: 13px; line-height: 1.6;
`

const DbErrorMsg = styled.p`
  padding: 12px 16px; border-radius: 9px; background: #fee2e2; color: #991b1b; font-size: 13px; margin-bottom: 12px;
`

const DbSavedMsg = styled.span`font-size: 12px; color: #10b981; font-weight: 600;`

const LegendRow = styled.div`display: flex; gap: 16px; margin-bottom: 16px; flex-wrap: wrap;`

const LegendItem = styled.div`display: flex; align-items: center; gap: 6px; font-size: 11px; color: #6b7280;`

const LegendDot = styled.span<{ $color: string }>`
  width: 8px; height: 8px; border-radius: 50%; background: ${({ $color }) => $color};
`

// ─── Composant entrée journal ─────────────────────────────────────────────────

// Clés contenant du contenu textuel utile (toutes sources)
const TEXT_KEYS = new Set([
  'plain_text', 'content', 'text', 'title', 'name', 'body', 'description',
  'summary', 'snippet', 'message', 'subject', 'label', 'value', 'display_name',
  'url', 'html_url', 'web_url', 'created_time', 'last_edited_time',
  'created_at', 'updated_at', 'closed_at', 'state', 'status', 'number',
])

// Clés à ignorer complètement (métadonnées techniques)
const SKIP_KEYS = new Set([
  'id', 'object', 'type', 'color', 'href', 'annotations', 'bold', 'italic',
  'strikethrough', 'underline', 'code', 'created_by', 'last_edited_by',
  'cover', 'icon', 'in_trash', 'is_archived', 'is_locked', 'public_url',
  'archived', 'link', 'has_children', 'parent', 'user', 'workspace',
  'node_id', 'sha', 'permissions', 'owner', 'private', 'fork', 'forks_count',
  'stargazers_count', 'watchers_count', 'open_issues_count', 'default_branch',
  'annotations_count', 'format',
])

// Détecte si une string ressemble à un UUID / ID technique
function looksLikeId(s: string): boolean {
  return /^[0-9a-f-]{32,}$/i.test(s) || /^[0-9a-f]{8}-[0-9a-f]{4}-/i.test(s)
}

function extractTextFromJson(obj: unknown, depth = 0): string[] {
  if (depth > 10) return []
  const lines: string[] = []

  if (typeof obj === 'string') {
    if (obj.length > 2 && !looksLikeId(obj)) lines.push(obj)
    return lines
  }
  if (Array.isArray(obj)) {
    for (const item of obj) lines.push(...extractTextFromJson(item, depth + 1))
    return lines
  }
  if (obj && typeof obj === 'object') {
    const record = obj as Record<string, unknown>
    for (const [key, val] of Object.entries(record)) {
      if (SKIP_KEYS.has(key)) continue
      if (TEXT_KEYS.has(key)) {
        if (typeof val === 'string' && val.length > 0 && !looksLikeId(val)) {
          // Format dates ISO en lisible
          if ((key === 'created_time' || key === 'created_at') && val.includes('T')) {
            lines.push(`Créé : ${new Date(val).toLocaleString('fr-FR')}`)
          } else if ((key === 'last_edited_time' || key === 'updated_at') && val.includes('T')) {
            lines.push(`Modifié : ${new Date(val).toLocaleString('fr-FR')}`)
          } else if (key === 'url' || key === 'html_url' || key === 'web_url') {
            lines.push(`Lien : ${val}`)
          } else if (key === 'state' || key === 'status') {
            lines.push(`Statut : ${val}`)
          } else if (key === 'number') {
            lines.push(`N° ${val}`)
          } else {
            lines.push(val)
          }
        } else if (typeof val === 'number') {
          if (key === 'number') lines.push(`N° ${val}`)
        } else if (Array.isArray(val) || (val && typeof val === 'object')) {
          lines.push(...extractTextFromJson(val, depth + 1))
        }
      } else {
        lines.push(...extractTextFromJson(val, depth + 1))
      }
    }
  }
  return lines
}

function formatJournalData(raw: string): string {
  try {
    const parsed = JSON.parse(raw)
    // Nouveau format avec security actions : extraire le champ "text"
    const source = parsed?.text !== undefined ? parsed.text : parsed
    if (typeof source === 'string') return source
    const lines = extractTextFromJson(source)
    const seen = new Set<string>()
    const unique = lines.filter(l => {
      const t = l.trim()
      if (t.length < 2 || seen.has(t)) return false
      seen.add(t)
      return true
    })
    return unique.length > 0 ? unique.join('\n') : raw
  } catch {
    return raw
  }
}

type SecurityAction = { kind: 'block' | 'tokenize' | 'alias' | 'mask'; field: string; real_value: string; replaced_by: string }

function parseSecurityActions(raw?: string): SecurityAction[] {
  if (!raw) return []
  try {
    const parsed = JSON.parse(raw)
    if (Array.isArray(parsed?.security)) return parsed.security as SecurityAction[]
  } catch { /* ignore */ }
  return []
}

// Patterns fixes à mettre en évidence dans les données brutes
// (les versions numérotées [TOKEN masqué #N] etc. sont ajoutées dynamiquement depuis les actions)
const HIGHLIGHT_PATTERNS = [
  { pattern: '[bloqué]', display: undefined as string | undefined, color: '#dc2626', bg: '#fee2e2' },
]

function HighlightedText({ text, actions }: { text: string; actions: SecurityAction[] }) {
  // Construit la liste de tous les patterns à chercher (fixes + alias replaced_by + emails/phones numérotés)
  // Pour les tok_nm_xxx, on ajoute un `display` avec le numéro pour l'affichage uniquement
  const patterns: { pattern: string; display?: string; color: string; bg: string }[] = [...HIGHLIGHT_PATTERNS]

  // DB tokens — numérotés dans le display (#N) mais matchés sur la valeur exacte
  const dbTokenActions = actions.filter(a => a.kind === 'tokenize' && a.replaced_by.startsWith('tok_'))
  dbTokenActions.forEach((a, i) => {
    if (!patterns.find(p => p.pattern === a.replaced_by))
      patterns.push({ pattern: a.replaced_by, display: `${a.replaced_by} #${i + 1}`, color: '#d97706', bg: '#fef3c7' })
  })

  for (const a of actions) {
    if (!a.replaced_by || patterns.find(p => p.pattern === a.replaced_by)) continue
    if (a.kind === 'alias')    patterns.push({ pattern: a.replaced_by, color: '#059669', bg: '#d1fae5' })
    if (a.kind === 'mask')     patterns.push({ pattern: a.replaced_by, color: '#dc2626', bg: '#fee2e2' })
    if (a.kind === 'tokenize') patterns.push({ pattern: a.replaced_by, color: '#d97706', bg: '#fef3c7' })
  }

  // Split le texte sur tous les patterns et retourne des spans
  const parts: React.ReactNode[] = []
  let remaining = text
  let key = 0

  while (remaining.length > 0) {
    let earliest = -1
    let matchedPattern: typeof patterns[0] | null = null

    for (const p of patterns) {
      const idx = remaining.indexOf(p.pattern)
      if (idx !== -1 && (earliest === -1 || idx < earliest)) {
        earliest = idx
        matchedPattern = p
      }
    }

    if (earliest === -1 || !matchedPattern) {
      parts.push(remaining)
      break
    }

    if (earliest > 0) parts.push(remaining.slice(0, earliest))
    parts.push(
      <span key={key++} style={{
        fontWeight: 700, color: matchedPattern.color,
        background: matchedPattern.bg, borderRadius: 3, padding: '0 2px',
      }}>{matchedPattern.display ?? matchedPattern.pattern}</span>
    )
    remaining = remaining.slice(earliest + matchedPattern.pattern.length)
  }

  return <>{parts}</>
}

function JournalEntryRow({ entry }: {
  entry: { ts: number; tool: string; query: string; results: number; blocked: boolean; data?: string | Record<string, unknown> }
}) {
  const [expanded, setExpanded] = useState(false)
  const date = new Date(entry.ts * 1000)
  const time = date.toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' })
  const day  = date.toLocaleDateString('fr-FR', { day: '2-digit', month: '2-digit' })
  // Normalise data : string ou objet → toujours string pour les parsers
  const rawData = entry.data == null ? undefined
    : typeof entry.data === 'string' ? entry.data
    : JSON.stringify(entry.data)
  const secActions = parseSecurityActions(rawData)
  const displayData = rawData ? formatJournalData(rawData) : ''
  const hasContent = !!rawData && !entry.blocked
  const color = connectorColor(entry.tool)
  const { connector, action } = parseToolDisplay(entry.tool)

  return (
    <JournalRow
      $blocked={entry.blocked}
      $color={color}
      onClick={hasContent ? () => setExpanded(v => !v) : undefined}
      style={hasContent ? { cursor: 'pointer' } : undefined}
    >
      <JournalMeta>
        <JournalTime>{day} {time}</JournalTime>
        <JournalConnector $color={entry.blocked ? '#dc2626' : color}>{connector}</JournalConnector>
        {action && <JournalAction>{action}</JournalAction>}
        <JournalResultsBadge $blocked={entry.blocked}>
          {entry.blocked ? '⛔ bloqué' : `${entry.results} résultat${entry.results !== 1 ? 's' : ''}`}
        </JournalResultsBadge>
        {secActions.length > 0 && (
          <>
            {secActions.some(a => a.kind === 'tokenize') && <SecurityBadge $kind="tokenize">🔑 {secActions.filter(a => a.kind === 'tokenize').length} tokenisé</SecurityBadge>}
            {secActions.some(a => a.kind === 'alias')    && <SecurityBadge $kind="alias">👤 alias</SecurityBadge>}
            {secActions.some(a => a.kind === 'mask')     && <SecurityBadge $kind="mask">🔒 {secActions.filter(a => a.kind === 'mask').length} masqué</SecurityBadge>}
            {secActions.some(a => a.kind === 'block')    && <SecurityBadge $kind="block">🚫 bloqué</SecurityBadge>}
          </>
        )}
      </JournalMeta>
      {entry.query && <JournalQuery>{entry.query}</JournalQuery>}
      {expanded && displayData && (
        <DataBox><HighlightedText text={displayData} actions={secActions} /></DataBox>
      )}
      {expanded && secActions.length > 0 && (
        <SecurityBox>
          <SecurityBoxHeader>
            <Shield size={11} />
            Filtres de confidentialité appliqués
          </SecurityBoxHeader>
          {/* Groupe mask — rendu d'un type de valeur masquée en colonne compacte */}
          {(['[email masqué', '[téléphone masqué'] as const).map(prefix => {
            const items = secActions.filter(a => a.kind === 'mask' && a.replaced_by.startsWith(prefix))
            if (items.length === 0) return null
            const label = prefix === '[email masqué' ? 'Emails masqués' : 'Téléphones masqués'
            return (
              <SecurityGroup key={prefix}>
                <SecurityGroupTitle $color="#dc2626">
                  <ShieldOff size={11} />
                  {label}
                </SecurityGroupTitle>
                <SecurityMaskList>
                  {items.map((a, i) => {
                    const numMatch = a.replaced_by.match(/#(\d+)\]$/)
                    const num = numMatch ? `#${numMatch[1]}` : `#${i + 1}`
                    return (
                      <SecurityMaskItem key={i}>
                        <SecurityMaskNum>{num}</SecurityMaskNum>
                        <SecurityMaskEmail>{a.real_value}</SecurityMaskEmail>
                      </SecurityMaskItem>
                    )
                  })}
                </SecurityMaskList>
              </SecurityGroup>
            )
          })}
          {/* Groupes tokenize — un groupe par type de placeholder (TOKEN, CLÉ API, DB) */}
          {([
            { prefix: '[TOKEN masqué',    label: 'Tokens détectés' },
            { prefix: '[CLÉ API masquée', label: 'Clés API masquées' },
          ] as const).map(({ prefix, label }) => {
            const items = secActions.filter(a => a.kind === 'tokenize' && a.replaced_by.startsWith(prefix))
            if (items.length === 0) return null
            return (
              <SecurityGroup key={prefix}>
                <SecurityGroupTitle $color="#d97706">
                  <Key size={11} />
                  {label}
                </SecurityGroupTitle>
                <SecurityMaskList>
                  {items.map((a, i) => {
                    const numMatch = a.replaced_by.match(/#(\d+)\]$/)
                    const num = numMatch ? `#${numMatch[1]}` : `#${i + 1}`
                    return (
                      <SecurityMaskItem key={i}>
                        <SecurityMaskNum>{num}</SecurityMaskNum>
                        <SecurityMaskEmail>{a.real_value}</SecurityMaskEmail>
                      </SecurityMaskItem>
                    )
                  })}
                </SecurityMaskList>
              </SecurityGroup>
            )
          })}
          {/* Tokenize DB (tok_nm_xxx) — affiche champ + valeur originale → token */}
          {secActions.some(a => a.kind === 'tokenize' && a.replaced_by.startsWith('tok_')) && (
            <SecurityGroup>
              <SecurityGroupTitle $color="#d97706">
                <Key size={11} />
                Données tokenisées
              </SecurityGroupTitle>
              <SecurityMaskList>
                {secActions.filter(a => a.kind === 'tokenize' && a.replaced_by.startsWith('tok_')).map((a, i) => (
                  <SecurityRow key={i}>
                    <SecurityMaskNum>#{i + 1}</SecurityMaskNum>
                    <SecurityRealValue>{a.field ? `${a.field}: ` : ''}{a.real_value}</SecurityRealValue>
                    <SecurityArrow>→</SecurityArrow>
                    <SecurityReplaced $kind="tokenize">{a.replaced_by}</SecurityReplaced>
                  </SecurityRow>
                ))}
              </SecurityMaskList>
            </SecurityGroup>
          )}
          {/* Groupe alias */}
          {secActions.some(a => a.kind === 'alias') && (
            <SecurityGroup>
              <SecurityGroupTitle $color="#059669">
                <UserRound size={11} />
                Alias appliqués
              </SecurityGroupTitle>
              <SecurityMaskList>
                {secActions.filter(a => a.kind === 'alias').map((a, i) => (
                  <SecurityRow key={i}>
                    <SecurityRealValue>{a.real_value}</SecurityRealValue>
                    <SecurityArrow>→</SecurityArrow>
                    <SecurityReplaced $kind="alias">{a.replaced_by}</SecurityReplaced>
                  </SecurityRow>
                ))}
              </SecurityMaskList>
            </SecurityGroup>
          )}
          {/* Groupe block (SQL tokenizer) */}
          {secActions.some(a => a.kind === 'block') && (
            <SecurityGroup>
              <SecurityGroupTitle $color="#dc2626">
                <ShieldOff size={11} />
                Valeurs bloquées
              </SecurityGroupTitle>
              <SecurityMaskList>
                {secActions.filter(a => a.kind === 'block').map((a, i) => (
                  <SecurityRow key={i}>
                    <SecurityRealValue>{a.real_value}</SecurityRealValue>
                    <SecurityArrow>→</SecurityArrow>
                    <SecurityReplaced $kind="block">{a.replaced_by}</SecurityReplaced>
                  </SecurityRow>
                ))}
              </SecurityMaskList>
            </SecurityGroup>
          )}
        </SecurityBox>
      )}
    </JournalRow>
  )
}

// ─── Composant carte action ───────────────────────────────────────────────────

function ActionCardItem({ action, onDecision }: {
  action: ActionRequest
  onDecision: () => void
}) {
  const [loading, setLoading] = useState(false)
  const [now, setNow] = useState(() => Math.floor(Date.now() / 1000))

  useEffect(() => {
    if (action.status !== 'pending') return
    const t = setInterval(() => setNow(Math.floor(Date.now() / 1000)), 1000)
    return () => clearInterval(t)
  }, [action.status])

  async function approve() {
    setLoading(true)
    try { await api.approveAction(action.id) } finally { setLoading(false); onDecision() }
  }
  async function reject() {
    setLoading(true)
    try { await api.rejectAction(action.id) } finally { setLoading(false); onDecision() }
  }

  // Parse source + tool name
  const rawTool = action.tool
  let source = ''
  let toolDisplay = ''
  if (rawTool.includes(':')) {
    const [s, t] = rawTool.split(':', 2)
    source = s.charAt(0).toUpperCase() + s.slice(1)
    toolDisplay = t.replace(/_/g, ' ')
  } else if (rawTool.startsWith('act_')) {
    toolDisplay = rawTool.replace('act_', '').replace(/_/g, ' ')
  } else {
    const parts = rawTool.split('_')
    source = parts[0].charAt(0).toUpperCase() + parts[0].slice(1)
    toolDisplay = parts.slice(1).join(' ')
  }

  // Parse params — show as key/value si JSON, sinon preview brute
  const params = action.params as Record<string, unknown>
  const paramEntries = Object.entries(params).filter(([, v]) => v !== undefined && v !== '' && v !== null)

  const date = new Date(action.created_at * 1000).toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' })
  const dateDay = new Date(action.created_at * 1000).toLocaleDateString('fr-FR', { day: '2-digit', month: '2-digit' })
  const expiresIn = Math.max(0, action.expires_at - now)
  const urgent = expiresIn < 60

  return (
    <ActionCard $status={action.status}>
      <CardTop>
        <CardTopLeft>
          {source && <SourceBadge>{source}</SourceBadge>}
          <ToolName>{toolDisplay || rawTool}</ToolName>
        </CardTopLeft>
        <StatusPill $status={action.status}>{StatusLabel[action.status] ?? action.status}</StatusPill>
      </CardTop>

      {paramEntries.length > 0 ? (
        <ParamsGrid>
          {paramEntries.map(([k, v]) => (
            <ParamRow key={k}>
              <ParamKey>{k}</ParamKey>
              <ParamVal>{typeof v === 'object' ? JSON.stringify(v) : String(v)}</ParamVal>
            </ParamRow>
          ))}
        </ParamsGrid>
      ) : (
        <PreviewBox>{action.preview}</PreviewBox>
      )}

      <CardFooter>
        <CardDate>{dateDay} à {date}</CardDate>
        {action.status === 'pending' && expiresIn > 0 && (
          <TimerBadge $urgent={urgent}>
            {urgent ? `⚠ ${expiresIn}s` : `${Math.ceil(expiresIn / 60)} min`}
          </TimerBadge>
        )}
      </CardFooter>

      {action.execution_result && (
        <ExecResult $ok={action.execution_result.startsWith('ok:')}>
          {action.execution_result.startsWith('ok:') ? '✓ ' : '✕ '}
          {action.execution_result.replace(/^(ok|err): /, '')}
        </ExecResult>
      )}

      {action.status === 'pending' && (
        <ActionBtns>
          <ApproveBtn onClick={approve} disabled={loading}>
            <span>✓</span> Approuver
          </ApproveBtn>
          <RejectBtn onClick={reject} disabled={loading}>
            <span>✕</span> Rejeter
          </RejectBtn>
        </ActionBtns>
      )}
    </ActionCard>
  )
}

// ─── Page principale ──────────────────────────────────────────────────────────

export default function ActionsPage() {
  type Section = 'flux' | 'sources' | 'privacy' | 'database'
  const [activeSection, setActiveSection] = useState<Section>('flux')
  const [sseConnected, setSseConnected] = useState(false)
  const queryClient = useQueryClient()
  const esRef = useRef<EventSource | null>(null)

  // ── Permissions MCP ─────────────────────────────────────────────────────
  const { data: permsData } = useQuery({
    queryKey: ['permissions'],
    queryFn:  api.getPermissions,
  })
  const [perms, setPerms] = useState<Record<string, boolean>>({})
  useEffect(() => { if (permsData) setPerms(permsData) }, [permsData])

  function togglePerm(key: string) {
    const next = { ...perms, [key]: !perms[key] }
    setPerms(next)
    api.savePermissions(next).then(() => queryClient.invalidateQueries({ queryKey: ['permissions'] }))
  }

  // ── Accès sources MCP ───────────────────────────────────────────────────
  const { data: sourceData } = useQuery({ queryKey: ['source-access'], queryFn: api.getSourceAccess })
  const [sources, setSources] = useState<Record<string, boolean>>({})
  useEffect(() => { if (sourceData) setSources(sourceData) }, [sourceData])

  function toggleSource(key: string) {
    const next = { ...sources, [key]: !sources[key] }
    setSources(next)
    api.saveSourceAccess(next).then(() => queryClient.invalidateQueries({ queryKey: ['source-access'] }))
  }

  // ── Config cloud connectors ──────────────────────────────────────────────
  const { data: configData } = useQuery({
    queryKey: ['config'],
    queryFn:  api.getConfig,
    enabled:  activeSection === 'sources',
  })

  // ── Alias Engine ────────────────────────────────────────────────────────
  type AliasEntry = { real: string; alias: string; alias_type?: string }
  const { data: serverAliasData } = useQuery({ queryKey: ['aliases'], queryFn: api.getAliases })
  const [aliases, setAliases] = useState<AliasEntry[]>([])
  const [aliasTypes, setAliasTypes] = useState<string[]>([])
  const [aliasesDirty, setAliasesDirty] = useState(false)
  const [selectedType, setSelectedType] = useState<string | null>(null)
  const [newReal, setNewReal] = useState('')
  const [newAlias, setNewAlias] = useState('')
  const [newTypeName, setNewTypeName] = useState('')
  const [showAddType, setShowAddType] = useState(false)
  useEffect(() => {
    if (serverAliasData !== undefined && !aliasesDirty) {
      setAliases(serverAliasData.aliases)
      setAliasTypes(serverAliasData.types)
      if (serverAliasData.types.length > 0) setSelectedType(t => t ?? serverAliasData.types[0])
    }
  }, [serverAliasData])
  const { mutate: saveAliases, isPending: savingAliases } = useMutation({
    mutationFn: (payload: { aliases: AliasEntry[]; types: string[] }) =>
      api.saveAliases(payload.aliases, payload.types),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['aliases'] })
      setAliasesDirty(false)
    },
  })

  function persistNow(nextAliases: AliasEntry[], nextTypes: string[]) {
    saveAliases({ aliases: nextAliases, types: nextTypes })
  }

  function addAlias() {
    const real = newReal.trim(); const alias = newAlias.trim()
    // Doublon uniquement au sein du même type (pas global)
    if (!real || !alias || aliases.some(a => a.real === real && a.alias_type === (selectedType ?? undefined))) return
    const next = [...aliases, { real, alias, alias_type: selectedType ?? undefined }]
    setAliases(next)
    setAliasesDirty(true)
    setNewReal(''); setNewAlias('')
    persistNow(next, aliasTypes)
  }
  function removeAlias(real: string, aliasType?: string) {
    const next = aliases.filter(a => !(a.real === real && a.alias_type === aliasType))
    setAliases(next)
    persistNow(next, aliasTypes)
  }
  function addType() {
    const t = newTypeName.trim()
    if (!t || aliasTypes.includes(t)) return
    const nextTypes = [...aliasTypes, t]
    setAliasTypes(nextTypes)
    setNewTypeName(''); setShowAddType(false)
    if (selectedType === null) setSelectedType(t)
    persistNow(aliases, nextTypes)
  }
  function deleteType(t: string) {
    const nextTypes = aliasTypes.filter(x => x !== t)
    const nextAliases = aliases.map(a => a.alias_type === t ? { ...a, alias_type: undefined } : a)
    setAliasTypes(nextTypes)
    setAliases(nextAliases)
    if (selectedType === t) setSelectedType(nextTypes[0] ?? null)
    persistNow(nextAliases, nextTypes)
  }

  // ── DB Security ──────────────────────────────────────────────────────────
  type SupabaseProject = { id: string; name: string; region: string }
  const [dbSecurity, setDbSecurity]   = useState<DbSecurityConfig>({ supabase: {} })
  const [dbSchemaLoading, setDbSchemaLoading] = useState(false)
  const [dbSchemaError, setDbSchemaError]     = useState<string | null>(null)
  const [dbSchemaTables, setDbSchemaTables]   = useState<Array<{ table_name: string; columns: Array<{ column_name: string; data_type: string }> }>>([])
  const [dbSaved, setDbSaved]                 = useState(false)
  const [dbProjects, setDbProjects]           = useState<SupabaseProject[]>([])
  const [dbProjectsLoading, setDbProjectsLoading] = useState(false)
  const [dbActiveProject, setDbActiveProject] = useState<string>('')

  const { data: dbSecurityData } = useQuery({
    queryKey: ['db-security'],
    queryFn:  api.getDbSecurity,
    enabled:  activeSection === 'database',
  })

  // Charge les projets + config sauvegardée à l'ouverture du tab
  useEffect(() => {
    if (activeSection !== 'database') return
    setDbProjectsLoading(true)
    Promise.all([api.getSupabaseProjects(), api.getDbSecurity()])
      .then(([projects, security]) => {
        setDbProjects(projects)
        const saved = security.active_project_id ?? ''
        const match = projects.find(p => p.id === saved)
        setDbActiveProject(match ? match.id : (projects[0]?.id ?? ''))
      })
      .catch(() => {})
      .finally(() => setDbProjectsLoading(false))
  }, [activeSection])

  useEffect(() => {
    if (dbSecurityData) {
      setDbSecurity(dbSecurityData)
      if (dbSchemaTables.length === 0 && Object.keys(dbSecurityData.supabase).length > 0) {
        const order = dbSecurityData.column_order ?? {}
        const synthetic = Object.keys(dbSecurityData.supabase)
          .sort((a, b) => a.localeCompare(b))
          .map(table_name => {
            const cols = dbSecurityData.supabase[table_name]
            const colNames = order[table_name]
              ? order[table_name].filter(c => c in cols)
              : Object.keys(cols).sort()
            return {
              table_name,
              columns: colNames.map((column_name, i) => ({ column_name, data_type: '', ordinal_position: i + 1 })),
            }
          })
        setDbSchemaTables(synthetic)
      }
    }
  }, [dbSecurityData])

  async function importSchema(projectId?: string) {
    setDbSchemaLoading(true); setDbSchemaError(null)
    try {
      if (projectId) await api.saveSupabaseProject(projectId)
      const tables = (await api.getSupabaseSchema()).sort((a, b) => a.table_name.localeCompare(b.table_name))
      setDbSchemaTables(tables)
      setDbSecurity(prev => {
        const targetId = projectId ?? prev.active_project_id ?? ''
        // Sauvegarder la config du projet actuel avant de switcher
        const updatedProjects = { ...(prev.projects ?? {}) }
        if (prev.active_project_id && prev.active_project_id !== targetId) {
          updatedProjects[prev.active_project_id] = {
            supabase: prev.supabase,
            column_order: prev.column_order,
          }
        }
        // Récupérer la config existante du projet cible (si déjà configuré)
        const existingProj = updatedProjects[targetId]
        const newSupabase: Record<string, Record<string, import('../api').ColumnRule>> = {}
        const newColumnOrder: Record<string, string[]> = {}
        for (const t of tables) {
          newColumnOrder[t.table_name] = t.columns.map(c => c.column_name)
          newSupabase[t.table_name] = {}
          for (const c of t.columns) {
            // Conserver la règle existante, sinon free
            newSupabase[t.table_name][c.column_name] =
              existingProj?.supabase?.[t.table_name]?.[c.column_name] ?? 'free'
          }
        }
        updatedProjects[targetId] = { supabase: newSupabase, column_order: newColumnOrder }
        const next: DbSecurityConfig = {
          active_project_id: targetId,
          supabase: newSupabase,
          column_order: newColumnOrder,
          projects: updatedProjects,
        }
        api.saveDbSecurity(next).catch(() => {})
        return next
      })
    } catch (e: unknown) {
      const projectName = dbProjects.find(p => p.id === (projectId ?? dbActiveProject))?.name ?? projectId ?? ''
      const base = e instanceof Error ? e.message : 'Erreur inconnue'
      setDbSchemaError(projectName ? `${projectName} — ${base}` : base)
      throw e
    } finally {
      setDbSchemaLoading(false)
    }
  }

  async function onProjectChange(projectId: string) {
    const previous = dbActiveProject
    setDbActiveProject(projectId)
    // Sauvegarder le projet sélectionné immédiatement (même si l'import échoue)
    api.saveDbSecurity({ ...dbSecurity, active_project_id: projectId }).catch(() => {})
    try {
      await importSchema(projectId)
    } catch {
      setDbActiveProject(previous)
      api.saveDbSecurity({ ...dbSecurity, active_project_id: previous }).catch(() => {})
    }
  }

  async function deleteSchema() {
    setDbSchemaTables([])
    const projId = dbSecurity.active_project_id
    const newProjects = { ...(dbSecurity.projects ?? {}) }
    if (projId) delete newProjects[projId]
    const empty: DbSecurityConfig = { supabase: {}, active_project_id: projId, projects: newProjects }
    setDbSecurity(empty)
    await api.saveDbSecurity(empty)
  }

  async function setColumnRule(table: string, column: string, rule: ColumnRule) {
    const newSupabase = {
      ...dbSecurity.supabase,
      [table]: { ...dbSecurity.supabase[table], [column]: rule },
    }
    const projId = dbSecurity.active_project_id
    const next: DbSecurityConfig = {
      active_project_id: projId,
      column_order: dbSecurity.column_order,
      supabase: newSupabase,
      projects: {
        ...(dbSecurity.projects ?? {}),
        ...(projId ? { [projId]: { supabase: newSupabase, column_order: dbSecurity.column_order } } : {}),
      },
    }
    setDbSecurity(next)
    try {
      await api.saveDbSecurity(next)
      setDbSaved(true)
      setTimeout(() => setDbSaved(false), 1500)
    } catch { /* ignore */ }
  }

  // ── SSE pour mises à jour temps réel ────────────────────────────────────
  useEffect(() => {
    const es = new EventSource('/api/actions/stream')
    esRef.current = es
    es.onopen  = () => setSseConnected(true)
    es.onerror = () => setSseConnected(false)
    es.onmessage = (e) => {
      try {
        const event = JSON.parse(e.data) as ActionEvent
        if (event.kind === 'new' || event.kind === 'updated') {
          queryClient.invalidateQueries({ queryKey: ['actions-pending'] })
          queryClient.invalidateQueries({ queryKey: ['actions-all'] })
        }
      } catch { /* ignore */ }
    }
    return () => { es.close(); setSseConnected(false) }
  }, [queryClient])

  const { data: pending = [], isLoading: loadingPending } = useQuery({
    queryKey: ['actions-pending'],
    queryFn:  api.getActionsPending,
    refetchInterval: 10_000,
  })

  const { data: all = [], isLoading: loadingAll } = useQuery({
    queryKey: ['actions-all'],
    queryFn:  api.getActionsAll,
    refetchInterval: false,
  })

  const { data: auditEntries = [], isLoading: loadingAudit } = useQuery({
    queryKey: ['audit'],
    queryFn:  () => api.getAudit(200),
    refetchInterval: 5_000,
  })

  function invalidate() {
    queryClient.invalidateQueries({ queryKey: ['actions-pending'] })
    queryClient.invalidateQueries({ queryKey: ['actions-all'] })
  }

  const nowTs = Math.floor(Date.now() / 1000)
  const history = all.filter(a => a.status !== 'pending')
  const visiblePending = pending.filter(a => a.expires_at > nowTs)

  const NAV_ITEMS = [
    { id: 'flux',      label: 'Flux d\'actions',   Icon: Zap      },
    { id: 'sources',   label: 'Sources',            Icon: PlugZap  },
    { id: 'privacy',   label: 'Confidentialité',    Icon: Shield   },
    { id: 'database',  label: 'Bases de données',   Icon: Database },
  ] as const

  return (
    <Layout>

      {/* ── Tab bar horizontale ── */}
      <TopTabBar>
        {NAV_ITEMS.map(({ id, label, Icon }) => (
          <TopTabItem
            key={id}
            $active={activeSection === id}
            onClick={() => setActiveSection(id)}
          >
            <Icon size={14} />
            {label}
            {id === 'flux' && visiblePending.length > 0 && <BadgeCount>{visiblePending.length}</BadgeCount>}
          </TopTabItem>
        ))}
        <SseStatusInline>
          <LiveDot $active={sseConnected} />
          {sseConnected ? 'Temps réel actif' : 'Connexion...'}
        </SseStatusInline>
      </TopTabBar>

      {/* ── Contenu ── */}
      <Content>

        {/* 1. Flux d'actions */}
        {activeSection === 'flux' && (
          <ActionsBlock>
            <ContentHeader>
              <PageTitle>Flux d'actions</PageTitle>
            </ContentHeader>
            <div style={{ display: 'flex', gap: 0, alignItems: 'flex-start', background: '#fff', border: '1px solid #e8eaed', borderRadius: 14, overflow: 'hidden' }}>

              {/* Colonne En attente */}
              <div style={{ flex: 1, minWidth: 0, padding: '16px 20px', borderRight: '1px solid #e8eaed' }}>
                <div style={{ fontSize: 12, fontWeight: 600, color: '#6b7280', textTransform: 'uppercase', letterSpacing: '.06em', marginBottom: 10, display: 'flex', alignItems: 'center', gap: 6 }}>
                  En attente
                  {visiblePending.length > 0 && <BadgeCount>{visiblePending.length}</BadgeCount>}
                </div>
                {loadingPending && <Loader />}
                {!loadingPending && visiblePending.length === 0 && (
                  <EmptyMsg>Aucune action en attente.<br />Ton client IA soumettra ici ses demandes d'actions pour validation.</EmptyMsg>
                )}
                <CardList>{visiblePending.map(a => <ActionCardItem key={a.id} action={a} onDecision={invalidate} />)}</CardList>
              </div>

              {/* Colonne Historique */}
              <div style={{ flex: 1, minWidth: 0, padding: '16px 20px', borderRight: '1px solid #e8eaed' }}>
                <div style={{ fontSize: 12, fontWeight: 600, color: '#6b7280', textTransform: 'uppercase', letterSpacing: '.06em', marginBottom: 10 }}>
                  Historique
                </div>
                {loadingAll && <Loader />}
                {!loadingAll && history.length === 0 && <EmptyMsg>Aucune action dans l'historique.</EmptyMsg>}
                <CardList>{history.map(a => <ActionCardItem key={a.id} action={a} onDecision={invalidate} />)}</CardList>
              </div>

              {/* Colonne Journal d'accès */}
              <div style={{ flex: 1, minWidth: 0, padding: '16px 20px' }}>
                <div style={{ fontSize: 12, fontWeight: 600, color: '#6b7280', textTransform: 'uppercase', letterSpacing: '.06em', marginBottom: 10 }}>
                  Journal d'accès
                </div>
                {loadingAudit && <Loader />}
                {!loadingAudit && auditEntries.length === 0 && (
                  <EmptyMsg>Aucune activité enregistrée.<br />Le journal se remplit dès que ton client IA utilise un tool OSMOzzz.</EmptyMsg>
                )}
                <JournalList>{auditEntries.map((e, i) => <JournalEntryRow key={i} entry={e} />)}</JournalList>
              </div>

            </div>
          </ActionsBlock>
        )}

        {/* 2. Sources */}
        {activeSection === 'sources' && (() => {
          // Sources locales — toujours présentes
          const staticRows: { key: string; label: string; hint: string }[] = [
            { key: 'email',    label: 'Gmail',      hint: 'Emails IMAP indexés'      },
            { key: 'imessage', label: 'iMessage',   hint: 'SMS et iMessages'          },
            { key: 'chrome',   label: 'Chrome',     hint: 'Historique de navigation'  },
            { key: 'safari',   label: 'Safari',     hint: 'Historique de navigation'  },
            { key: 'notes',    label: 'Notes',      hint: 'Apple Notes'               },
            { key: 'calendar', label: 'Calendrier', hint: 'Apple Calendar'            },
            { key: 'terminal', label: 'Terminal',   hint: 'Historique zsh'            },
            { key: 'file',     label: 'Fichiers',   hint: 'Desktop & Documents'       },
            { key: 'notion',   label: 'Notion',     hint: 'Pages indexées'            },
            { key: 'github',   label: 'GitHub',     hint: 'Issues & PRs indexés'      },
            { key: 'linear',   label: 'Linear',     hint: 'Issues indexées'           },
            { key: 'jira',     label: 'Jira',       hint: 'Tickets indexés'           },
          ]
          // Cloud connectors — affichés seulement si configurés
          const CLOUD_META: Record<string, { label: string; hint: string }> = {
            slack:      { label: 'Slack',       hint: 'Messages indexés'            },
            trello:     { label: 'Trello',      hint: 'Cartes indexées'             },
            todoist:    { label: 'Todoist',     hint: 'Tâches indexées'             },
            gitlab:     { label: 'GitLab',      hint: 'Issues & MRs indexées'       },
            airtable:   { label: 'Airtable',    hint: 'Bases indexées'              },
            obsidian:   { label: 'Obsidian',    hint: 'Vault local indexé'          },
            supabase:   { label: 'Supabase',    hint: 'Base de données cloud'       },
            sentry:     { label: 'Sentry',      hint: 'Erreurs & alertes'           },
            cloudflare: { label: 'Cloudflare',  hint: 'DNS, Workers, Pages'         },
            vercel:     { label: 'Vercel',      hint: 'Déploiements & projets'      },
            railway:    { label: 'Railway',     hint: 'Services & déploiements'     },
            render:     { label: 'Render',      hint: 'Services cloud'              },
            google:     { label: 'Google',      hint: 'Workspace (Calendar, Drive)' },
            stripe:     { label: 'Stripe',      hint: 'Paiements & clients'         },
            hubspot:    { label: 'HubSpot',     hint: 'CRM & marketing'             },
            posthog:    { label: 'PostHog',     hint: 'Analytics produit'           },
            resend:     { label: 'Resend',      hint: 'Emails transactionnels'      },
            discord:    { label: 'Discord',     hint: 'Serveur & messages'          },
            twilio:     { label: 'Twilio',      hint: 'SMS & voix'                  },
            figma:      { label: 'Figma',       hint: 'Fichiers & composants'       },
          }
          const staticKeys = new Set(staticRows.map(r => r.key))
          const dynamicRows = configData
            ? Object.entries(CLOUD_META)
                .filter(([key]) => !staticKeys.has(key) && (configData as unknown as Record<string, { configured: boolean } | undefined>)[key]?.configured)
                .map(([key, meta]) => ({ key, ...meta }))
            : []
          const allRows = [...staticRows, ...dynamicRows]
          return (
            <>
              <ContentHeader>
                <PageTitle>Contrôle des sources</PageTitle>
              </ContentHeader>
              <PermSection>
                <PermHeader>
                  <PermTitle>Accès et validation par source</PermTitle>
                  <PermDesc>Contrôlez quelles sources sont accessibles à votre client IA et lesquelles nécessitent une validation manuelle avant exécution.</PermDesc>
                </PermHeader>
                <div style={{ padding: '8px 0' }}>
                  <SourceTable>
                    <thead>
                      <tr>
                        <SourceTh>Source</SourceTh>
                        <SourceTh $center>Accès client IA</SourceTh>
                        <SourceTh $center>Validation manuelle</SourceTh>
                      </tr>
                    </thead>
                    <tbody>
                      {allRows.map(({ key, label, hint }) => (
                        <tr key={key}>
                          <SourceTd><PermLabel>{label}</PermLabel><PermHint>{hint}</PermHint></SourceTd>
                          <SourceTd $center>
                            <Toggle $on={sources[key] !== false} onClick={() => toggleSource(key)} />
                          </SourceTd>
                          <SourceTd $center>
                            <Toggle $on={!!perms[key]} onClick={() => togglePerm(key)} />
                          </SourceTd>
                        </tr>
                      ))}
                    </tbody>
                  </SourceTable>
                </div>
              </PermSection>
            </>
          )
        })()}

        {/* 3. Confidentialité + Alias */}
        {activeSection === 'privacy' && (
          <>
            <ContentHeader>
              <PageTitle>Confidentialité</PageTitle>
            </ContentHeader>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 20 }}>
              <PrivacyPanel />
              <PermSection>
                <PermHeader>
                  <PermTitle>Alias d'identité</PermTitle>
                  <PermDesc>Remplace les vrais noms par des alias avant envoi à votre client IA. Organisez vos alias par type.</PermDesc>
                </PermHeader>
                <div style={{ display: 'flex', minHeight: 0 }}>
                  {/* Mini sidebar types */}
                  <div style={{ width: 160, borderRight: '1px solid #f3f4f6', padding: '12px 8px', display: 'flex', flexDirection: 'column', gap: 2 }}>
                    {aliasTypes.map(t => (
                      <div key={t} style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
                        <SideNavItem $active={selectedType === t} onClick={() => setSelectedType(t)} style={{ fontSize: 12, padding: '6px 10px', flex: 1 }}>
                          {t} ({aliases.filter(a => a.alias_type === t).length})
                        </SideNavItem>
                        <AliasDelBtn onClick={() => deleteType(t)} style={{ padding: '2px 6px', fontSize: 10 }}>✕</AliasDelBtn>
                      </div>
                    ))}
                    {showAddType ? (
                      <div style={{ padding: '6px 4px', display: 'flex', flexDirection: 'column', gap: 4 }}>
                        <AliasInput placeholder="Nom du type" value={newTypeName}
                          onChange={e => setNewTypeName(e.target.value)}
                          onKeyDown={e => { if (e.key === 'Enter') addType(); if (e.key === 'Escape') setShowAddType(false) }}
                          style={{ fontSize: 12, padding: '5px 8px' }} autoFocus />
                        <AliasAddBtn onClick={addType} disabled={!newTypeName.trim()} style={{ fontSize: 11, padding: '4px 8px' }}>OK</AliasAddBtn>
                      </div>
                    ) : (
                      <button onClick={() => setShowAddType(true)} style={{ background: 'none', border: '1px dashed #d1d5db', borderRadius: 7, padding: '5px 10px', fontSize: 11, color: '#9ca3af', cursor: 'pointer', marginTop: 4, textAlign: 'left' }}>
                        + Nouveau type
                      </button>
                    )}
                  </div>
                  {/* Contenu aliases */}
                  <div style={{ flex: 1, padding: '12px 16px', minWidth: 0 }}>
                    <AliasTable>
                      <thead>
                        <tr>
                          <AliasTh>Vrai nom</AliasTh>
                          <AliasTh style={{ width: 24 }} />
                          <AliasTh>Alias</AliasTh>
                          <AliasTh style={{ width: 80 }} />
                        </tr>
                      </thead>
                      <tbody>
                        {aliases.filter(a => selectedType === null || a.alias_type === selectedType).length === 0 && (
                          <tr><td colSpan={5} style={{ textAlign: 'center', padding: '20px', color: '#9ca3af', fontSize: 13 }}>Aucun alias{selectedType ? ` dans "${selectedType}"` : ''}.</td></tr>
                        )}
                        {aliases
                          .filter(a => selectedType === null || a.alias_type === selectedType)
                          .map(({ real, alias, alias_type }) => (
                            <tr key={`${real}__${alias_type ?? ''}`}>
                              <AliasTd><strong>{real}</strong></AliasTd>
                              <AliasArrow>→</AliasArrow>
                              <AliasMuted>{alias}</AliasMuted>
                              <AliasTd style={{ textAlign: 'right' }}>
                                <AliasDelBtn onClick={() => removeAlias(real, alias_type)}>
                                  Supprimer
                                </AliasDelBtn>
                              </AliasTd>
                            </tr>
                          ))}
                      </tbody>
                    </AliasTable>
                    <AliasAddRow style={{ marginTop: 12 }}>
                      <AliasInput placeholder="Vrai nom" value={newReal}
                        onChange={e => setNewReal(e.target.value)}
                        onKeyDown={e => e.key === 'Enter' && addAlias()} />
                      <span style={{ color: '#d1d5db', fontSize: 16 }}>→</span>
                      <AliasInput placeholder="Alias" value={newAlias}
                        onChange={e => setNewAlias(e.target.value)}
                        onKeyDown={e => e.key === 'Enter' && addAlias()} />
                      <AliasAddBtn onClick={addAlias} disabled={!newReal.trim() || !newAlias.trim() || savingAliases}>
                        {savingAliases ? '…' : 'Ajouter'}
                      </AliasAddBtn>
                    </AliasAddRow>
                  </div>
                </div>
              </PermSection>
            </div>
          </>
        )}

        {/* 5. Bases de données */}
        {activeSection === 'database' && (
          <>
            <ContentHeader>
              <PageTitle>Sécurité des bases de données</PageTitle>
            </ContentHeader>

            <DbToolbar>
              {dbProjectsLoading ? (
                <DbProjectBadge><RefreshCw size={12} style={{ animation: 'spin .7s linear infinite' }} /> Chargement…</DbProjectBadge>
              ) : dbProjects.length > 0 ? (
                <ProjectSelect
                  value={dbActiveProject}
                  onChange={e => onProjectChange(e.target.value)}
                  disabled={dbSchemaLoading}
                  style={{ width: 'auto', padding: '7px 12px', fontSize: 13 }}
                >
                  {dbProjects.map(p => (
                    <option key={p.id} value={p.id}>{p.name}</option>
                  ))}
                </ProjectSelect>
              ) : (
                <DbProjectBadge style={{ color: '#9ca3af', background: '#f9fafb', borderColor: '#e5e7eb' }}>
                  Aucun projet Supabase configuré
                </DbProjectBadge>
              )}

              {dbSchemaLoading && <DbProjectBadge><RefreshCw size={12} style={{ animation: 'spin .7s linear infinite' }} /> Importation…</DbProjectBadge>}

              {dbSchemaTables.length > 0 && !dbSchemaLoading && (
                <DbDeleteBtn onClick={deleteSchema}>Supprimer la structure</DbDeleteBtn>
              )}

              {dbSaved && <DbSavedMsg>✓ Sauvegardé</DbSavedMsg>}
            </DbToolbar>

            {dbSchemaError && <DbErrorMsg>Erreur : {dbSchemaError}</DbErrorMsg>}

            {dbSchemaTables.length > 0 && (
              <LegendRow>
                <LegendItem><LegendDot $color="#10b981" />Libre — valeur brute transmise au client IA</LegendItem>
                <LegendItem><LegendDot $color="#f59e0b" />Tokenisé — remplacé par un token stable (tok_em_…)</LegendItem>
                <LegendItem><LegendDot $color="#ef4444" />Bloqué — colonne visible mais valeur masquée ([bloqué])</LegendItem>
              </LegendRow>
            )}

            {dbSchemaTables.length === 0 && !dbSchemaLoading && dbProjects === null && (
              <DbTableCard>
                <DbEmptyMsg>
                  Aucune structure importée.<br />
                  Cliquez sur <strong>Importer la structure Supabase</strong> pour récupérer vos tables et configurer la sécurité colonne par colonne.
                </DbEmptyMsg>
              </DbTableCard>
            )}

            {dbSchemaTables.map(table => (
              <DbTableCard key={table.table_name}>
                <DbTableHeader>
                  <DbTableName>{table.table_name}</DbTableName>
                  <span style={{ fontSize: 11, color: '#9ca3af' }}>{table.columns.length} colonne{table.columns.length !== 1 ? 's' : ''}</span>
                </DbTableHeader>
                {table.columns.map(col => {
                  const rule: ColumnRule = dbSecurity.supabase[table.table_name]?.[col.column_name] ?? 'free'
                  return (
                    <DbColRow key={col.column_name}>
                      <DbColName>{col.column_name}</DbColName>
                      <DbColType>{col.data_type}</DbColType>
                      <RuleSelector>
                        <RuleBtn
                          $active={rule === 'free'} $variant="free"
                          onClick={() => setColumnRule(table.table_name, col.column_name, 'free')}
                        >Libre</RuleBtn>
                        <RuleBtn
                          $active={rule === 'tokenize'} $variant="tokenize"
                          onClick={() => setColumnRule(table.table_name, col.column_name, 'tokenize')}
                        >Tokenisé</RuleBtn>
                        <RuleBtn
                          $active={rule === 'block'} $variant="block"
                          onClick={() => setColumnRule(table.table_name, col.column_name, 'block')}
                        >Bloqué</RuleBtn>
                      </RuleSelector>
                    </DbColRow>
                  )
                })}
              </DbTableCard>
            ))}
          </>
        )}

      </Content>
    </Layout>
  )
}
