'use client'
import React, { useState, useEffect, useCallback } from 'react'
import styled, { createGlobalStyle } from 'styled-components'
import Link from 'next/link'

// ─── Global ───────────────────────────────────────────────────────────────────

const GlobalStyle = createGlobalStyle`
  *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
  :root {
    --bg:      #0a0b0f;
    --bg2:     #0f1117;
    --bg3:     #13151e;
    --border:  #1f2230;
    --text:    #e8eaf0;
    --muted:   #6b7280;
    --accent:  #5b5ef4;
    --accent-dim: rgba(91,94,244,.15);
    --green:   #4ade80;
  }
  html { scroll-behavior: smooth; }
  body {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
    background: var(--bg);
    color: var(--text);
    line-height: 1.6;
    -webkit-font-smoothing: antialiased;
  }
  * { scrollbar-width: thin; scrollbar-color: #1f2230 transparent; }
`

// ─── Layout ───────────────────────────────────────────────────────────────────

const Shell = styled.div`
  display: flex;
  flex-direction: column;
  min-height: 100vh;
`

const TopBar = styled.header`
  position: sticky;
  top: 0;
  z-index: 100;
  background: rgba(10,11,15,.92);
  backdrop-filter: blur(12px);
  border-bottom: 1px solid var(--border);
  height: 54px;
  display: flex;
  align-items: center;
  padding: 0 24px;
  gap: 16px;
`

const TopBarLogo = styled(Link)`
  display: flex;
  align-items: center;
  gap: 10px;
  text-decoration: none;
  color: var(--text);
  font-size: 15px;
  font-weight: 700;
  letter-spacing: -.02em;
  flex-shrink: 0;
`

const TopBarSep = styled.span`
  color: var(--border);
  font-size: 20px;
  font-weight: 200;
`

const TopBarLabel = styled.span`
  font-size: 13px;
  font-weight: 500;
  color: var(--muted);
`

const TopBarRight = styled.div`
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 12px;
`

const GhLink = styled.a`
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 5px 12px;
  border-radius: 8px;
  border: 1px solid var(--border);
  color: var(--muted);
  font-size: 12px;
  font-weight: 500;
  text-decoration: none;
  transition: border-color .15s, color .15s;
  &:hover { border-color: #374151; color: var(--text); }
`

const Body = styled.div`
  display: flex;
  flex: 1;
  width: 100%;
`

// ─── Sidebar ──────────────────────────────────────────────────────────────────

const Sidebar = styled.aside`
  width: 350px;
  flex-shrink: 0;
  position: sticky;
  top: 54px;
  height: calc(100vh - 54px);
  overflow-y: auto;
  padding: 40px;
  border-right: 1px solid var(--border);
`

const SideLogoBlock = styled.div`
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 16px 20px;
  margin-bottom: 4px;
`

const SideLogoName = styled.span`
  font-size: 15px;
  font-weight: 700;
  color: var(--accent);
  letter-spacing: -.01em;
`

const SideGroup = styled.div`
  margin-top: 4px;

`

const SideGroupSep = styled.div`
  height: 1px;
  background: var(--border);
  margin: 16px 0;
`

const SideGroupLabel = styled.div`
  font-size: 11px;
  font-weight: 800;
  text-transform: uppercase;
  letter-spacing: .1em;
  color: #e8eaf0;
  padding: 8px 16px 6px;
`

const SideItem = styled.a<{ $active?: boolean }>`
  display: block;
  padding: 7px 16px;
  font-size: 14px;
  font-weight: 400;
  color: ${p => p.$active ? '#fff' : '#9ca3af'};
  background: transparent;
  text-decoration: none;
  cursor: pointer;
  transition: color .12s;
  position: relative;
  &:hover { color: ${p => p.$active ? '#fff' : '#e8eaf0'}; }
  &::after {
    content: '';
    position: absolute;
    bottom: 2px;
    left: 16px;
    height: 1px;
    background: #fff;
    width: ${p => p.$active ? '100%' : '0%'};
    transform-origin: left;
    transition: width .3s cubic-bezier(.4,0,.2,1);
  }
`

// ─── Content ──────────────────────────────────────────────────────────────────

const Content = styled.main`
  flex: 1;
  min-width: 0;
  padding: 48px 80px 96px 64px;
  max-width: 1000px;
  margin: 0 auto;
`

const DocSection = styled.section`
  padding-top: 16px;
  margin-bottom: 72px;
  scroll-margin-top: 80px;
`

const DocH1 = styled.h1`
  font-size: 32px;
  font-weight: 800;
  letter-spacing: -.03em;
  color: #fff;
  margin-bottom: 16px;
`

const DocH2 = styled.h2`
  font-size: 22px;
  font-weight: 700;
  letter-spacing: -.02em;
  color: #fff;
  margin-top: 48px;
  margin-bottom: 12px;
  scroll-margin-top: 80px;
`

const DocH3 = styled.h3`
  font-size: 15px;
  font-weight: 600;
  color: #fff;
  margin-top: 28px;
  margin-bottom: 8px;
`

const DocP = styled.p`
  font-size: 14px;
  line-height: 1.8;
  color: #9ca3af;
  margin-bottom: 16px;
`

const DocDivider = styled.hr`
  border: none;
  border-top: 1px solid var(--border);
  margin: 48px 0;
`

// ─── Code block ───────────────────────────────────────────────────────────────

const CodeWrap = styled.div`
  position: relative;
  margin: 16px 0;
  border-radius: 12px;
  overflow: hidden;
  border: 1px solid var(--border);
`

const CodeHeader = styled.div`
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 16px;
  background: #13151e;
  border-bottom: 1px solid var(--border);
`

const CodeLang = styled.span`
  font-size: 11px;
  font-weight: 600;
  color: var(--muted);
  text-transform: uppercase;
  letter-spacing: .06em;
`

const CodeFilePath = styled.span`
  font-size: 11px;
  color: var(--muted);
  font-family: 'SF Mono', Monaco, monospace;
`

const CopyBtn = styled.button<{ $copied?: boolean }>`
  font-size: 11px;
  font-weight: 600;
  padding: 3px 10px;
  border-radius: 6px;
  border: 1px solid ${p => p.$copied ? 'rgba(74,222,128,.3)' : 'var(--border)'};
  background: ${p => p.$copied ? 'rgba(74,222,128,.08)' : 'transparent'};
  color: ${p => p.$copied ? 'var(--green)' : 'var(--muted)'};
  cursor: pointer;
  font-family: inherit;
  transition: all .15s;
  &:hover { border-color: #374151; color: var(--text); }
`

const Pre = styled.pre`
  background: #0d0e15;
  padding: 20px;
  overflow-x: auto;
  font-family: 'SF Mono', 'Fira Code', Monaco, monospace;
  font-size: 13px;
  line-height: 1.7;
  color: #a5b4fc;
`

const InlineCode = styled.code`
  font-family: 'SF Mono', 'Fira Code', Monaco, monospace;
  font-size: 12px;
  background: rgba(91,94,244,.12);
  color: #a5b4fc;
  padding: 2px 7px;
  border-radius: 5px;
`

// ─── Tabs (client selector) ───────────────────────────────────────────────────

const TabsBar = styled.div`
  display: flex;
  gap: 2px;
  background: var(--bg3);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 4px;
  margin-bottom: 16px;
  width: fit-content;
`

const Tab = styled.button<{ $active?: boolean }>`
  padding: 6px 14px;
  border-radius: 7px;
  border: none;
  background: ${p => p.$active ? 'var(--accent)' : 'transparent'};
  color: ${p => p.$active ? '#fff' : 'var(--muted)'};
  font-size: 13px;
  font-weight: ${p => p.$active ? '600' : '400'};
  cursor: pointer;
  font-family: inherit;
  transition: all .15s;
  &:hover { color: ${p => p.$active ? '#fff' : 'var(--text)'}; }
`

// ─── Step list ────────────────────────────────────────────────────────────────

const StepList = styled.ol`
  display: flex;
  flex-direction: column;
  gap: 16px;
  list-style: none;
  counter-reset: step;
`

const StepItem = styled.li`
  display: flex;
  gap: 16px;
  counter-increment: step;
  &::before {
    content: counter(step);
    display: flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    min-width: 26px;
    border-radius: 50%;
    background: var(--accent-dim);
    color: var(--accent);
    font-size: 12px;
    font-weight: 700;
    margin-top: 2px;
  }
`

const StepBody = styled.div`
  flex: 1;
  font-size: 14px;
  line-height: 1.7;
  color: #9ca3af;
  strong { color: #e8eaf0; font-weight: 600; }
`

// ─── Tool table ───────────────────────────────────────────────────────────────

const ToolTable = styled.table`
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
  margin: 16px 0;
`

const Th = styled.th`
  text-align: left;
  padding: 10px 14px;
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: .06em;
  color: var(--muted);
  border-bottom: 1px solid var(--border);
`

const Td = styled.td`
  padding: 10px 14px;
  border-bottom: 1px solid rgba(31,34,48,.8);
  color: #9ca3af;
  vertical-align: top;
  line-height: 1.5;
`

const TdCode = styled(Td)`
  font-family: 'SF Mono', 'Fira Code', Monaco, monospace;
  color: #a5b4fc;
  font-size: 12px;
`

const Badge = styled.span<{ $color?: string }>`
  font-size: 10px;
  font-weight: 700;
  padding: 2px 7px;
  border-radius: 4px;
  background: ${p => p.$color === 'green'
    ? 'rgba(74,222,128,.1)'
    : p.$color === 'purple'
    ? 'rgba(91,94,244,.1)'
    : 'rgba(107,114,128,.1)'};
  color: ${p => p.$color === 'green'
    ? '#4ade80'
    : p.$color === 'purple'
    ? '#a5b4fc'
    : '#9ca3af'};
  letter-spacing: .04em;
`

const ConnectorRow = styled.tr<{ $open: boolean }>`
  cursor: pointer;
  transition: background .12s;
  background: ${p => p.$open ? 'rgba(91,94,244,.06)' : 'transparent'};
  &:hover { background: rgba(91,94,244,.04); }
`

const ConnectorExpand = styled.tr`
  background: rgba(15,17,28,.6);
`

const ToolsGrid = styled.td`
  padding: 10px 14px 16px 14px;
  border-bottom: 1px solid rgba(31,34,48,.8);
`

const ToolPill = styled.span`
  display: inline-block;
  font-family: 'SF Mono', 'Fira Code', Monaco, monospace;
  font-size: 11px;
  color: #a5b4fc;
  background: rgba(91,94,244,.08);
  border: 1px solid rgba(91,94,244,.15);
  border-radius: 4px;
  padding: 2px 7px;
  margin: 3px 3px 0 0;
`

const CountBadge = styled.span`
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 28px;
  padding: 1px 7px;
  border-radius: 99px;
  font-size: 11px;
  font-weight: 700;
  background: rgba(91,94,244,.12);
  color: #a5b4fc;
`

const Chevron = styled.span<{ $open: boolean }>`
  display: inline-block;
  transition: transform .2s;
  transform: ${p => p.$open ? 'rotate(90deg)' : 'rotate(0deg)'};
  color: #4b5563;
  font-size: 11px;
`

// ─── Logo SVG ─────────────────────────────────────────────────────────────────

function SiteLogo({ size = 28 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 64 64" fill="none">
      <rect x="20" y="20" width="24" height="24" rx="1"
        stroke="rgba(255,255,255,0.35)" strokeWidth="0.6" />
      <path d="M 20 28 L 20 20 L 28 20" stroke="#fff" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
      <path d="M 36 20 L 44 20 L 44 28" stroke="#fff" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
      <path d="M 20 36 L 20 44 L 28 44" stroke="#fff" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
      <path d="M 44 36 L 44 44 L 36 44" stroke="#fff" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
      <circle cx="32" cy="32" r="2.5" fill="#fff" />
    </svg>
  )
}

function GithubIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
      <path d="M12 2C6.477 2 2 6.477 2 12c0 4.42 2.865 8.166 6.839 9.489.5.092.682-.217.682-.482 0-.237-.008-.866-.013-1.7-2.782.603-3.369-1.342-3.369-1.342-.454-1.155-1.11-1.463-1.11-1.463-.908-.62.069-.608.069-.608 1.003.07 1.531 1.03 1.531 1.03.892 1.529 2.341 1.087 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.11-4.555-4.943 0-1.091.39-1.984 1.029-2.683-.103-.253-.446-1.27.098-2.647 0 0 .84-.269 2.75 1.025A9.578 9.578 0 0112 6.836c.85.004 1.705.114 2.504.336 1.909-1.294 2.747-1.025 2.747-1.025.546 1.377.202 2.394.1 2.647.64.699 1.028 1.592 1.028 2.683 0 3.842-2.339 4.687-4.566 4.935.359.309.678.919.678 1.852 0 1.336-.012 2.415-.012 2.741 0 .267.18.578.688.48C19.138 20.163 22 16.418 22 12c0-5.523-4.477-10-10-10z" />
    </svg>
  )
}

// ─── Copy hook ────────────────────────────────────────────────────────────────

function useCopy() {
  const [copied, setCopied] = useState<string | null>(null)
  const copy = useCallback((text: string, id: string) => {
    navigator.clipboard.writeText(text)
    setCopied(id)
    setTimeout(() => setCopied(null), 2000)
  }, [])
  return { copied, copy }
}

// ─── Code Block component ─────────────────────────────────────────────────────

function CodeBlock({ code, lang = 'json', file, id }: { code: string; lang?: string; file?: string; id: string }) {
  const { copied, copy } = useCopy()
  return (
    <CodeWrap>
      <CodeHeader>
        <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
          <CodeLang>{lang}</CodeLang>
          {file && <CodeFilePath>{file}</CodeFilePath>}
        </div>
        <CopyBtn $copied={copied === id} onClick={() => copy(code, id)}>
          {copied === id ? '✓ Copié' : 'Copier'}
        </CopyBtn>
      </CodeHeader>
      <Pre>{code}</Pre>
    </CodeWrap>
  )
}

// ─── Data ─────────────────────────────────────────────────────────────────────

const MCP_CLIENTS = ['Claude Desktop', 'Cursor', 'Windsurf', 'Zed'] as const
type McpClient = typeof MCP_CLIENTS[number]

const MCP_CONFIGS: Record<McpClient, { file: string; code: string }> = {
  'Claude Desktop': {
    file: '~/Library/Application Support/Claude/claude_desktop_config.json',
    code: `{
  "mcpServers": {
    "osmozzz": {
      "command": "/usr/local/bin/osmozzz",
      "args": ["mcp"]
    }
  }
}`,
  },
  'Cursor': {
    file: '~/.cursor/mcp.json',
    code: `{
  "mcpServers": {
    "osmozzz": {
      "command": "/usr/local/bin/osmozzz",
      "args": ["mcp"]
    }
  }
}`,
  },
  'Windsurf': {
    file: '~/.codeium/windsurf/mcp_config.json',
    code: `{
  "mcpServers": {
    "osmozzz": {
      "command": "/usr/local/bin/osmozzz",
      "args": ["mcp"]
    }
  }
}`,
  },
  'Zed': {
    file: '~/.config/zed/settings.json',
    code: `{
  "context_servers": {
    "osmozzz": {
      "command": {
        "path": "/usr/local/bin/osmozzz",
        "args": ["mcp"]
      }
    }
  }
}`,
  },
}

const CONNECTORS: { name: string; count: number; tools: string[] }[] = [
  { name: 'Gmail',            count: 7,   tools: ['gmail_search','gmail_recent','gmail_read','gmail_by_sender','gmail_send','gmail_reply','gmail_stats'] },
  { name: 'GitHub',           count: 40,  tools: ['create_issue','get_issue','list_issues','search_issues','create_pull_request','get_pull_request','list_pull_requests','merge_pull_request','create_branch','push_files','get_file_contents','create_or_update_file','fork_repository','create_repository','search_repositories','search_code','search_users','add_issue_comment','list_commits','get_pull_request_status','get_pull_request_files','get_pull_request_reviews','get_pull_request_comments','create_pull_request_review','update_pull_request_branch','update_issue','...'] },
  { name: 'GitLab',           count: 135, tools: ['gitlab_list_issues','gitlab_get_issue','gitlab_create_issue','gitlab_update_issue','gitlab_close_issue','gitlab_add_comment','gitlab_list_mrs','gitlab_get_mr','gitlab_create_mr','gitlab_merge_mr','gitlab_list_pipelines','gitlab_get_pipeline','gitlab_retry_pipeline','gitlab_cancel_pipeline','gitlab_list_projects','gitlab_list_branches','gitlab_create_branch','gitlab_push_files','gitlab_fork_repository','gitlab_create_repository','create_merge_request_thread','list_merge_request_diffs','get_merge_request_conflicts','approve_merge_request','create_label','list_labels','create_milestone','list_milestones','create_release','list_releases','get_pipeline_job_output','...'] },
  { name: 'Linear',           count: 17,  tools: ['linear_search_issues','linear_get_issue','linear_create_issue','linear_update_issue','linear_add_comment','linear_list_teams','linear_list_issues','linear_list_projects','linear_list_workflow_states','linear_list_labels','linear_list_members','linear_archive_issue','linear_get_viewer','linear_create_project','linear_list_cycles','linear_get_cycle','linear_delete_comment'] },
  { name: 'Jira',             count: 23,  tools: ['jira_search_issues','jira_get_issue','jira_create_issue','jira_update_issue','jira_add_comment','jira_get_comments','jira_transition_issue','jira_list_transitions','jira_assign_issue','jira_list_projects','jira_get_issue_types','jira_list_priorities','jira_search_users','jira_add_worklog','jira_list_boards','jira_list_sprints','jira_delete_issue','jira_link_issues','jira_list_link_types','jira_get_current_user','jira_list_versions','jira_move_to_sprint','jira_get_fields'] },
  { name: 'Notion',           count: 22,  tools: ['notion_search','notion_get_page','notion_create_page','notion_update_page','notion_get_database','notion_query_database','notion_get_block_children','notion_append_block_children','notion_update_block','notion_delete_block','notion_get_user','notion_get_users','notion_get_self','notion_create_comment','notion_retrieve_comment','notion_create_data_source','notion_get_data_source','notion_update_data_source','notion_list_data_source_templates','notion_move_page','notion_retrieve_block','notion_retrieve_page_property'] },
  { name: 'Slack',            count: 50,  tools: ['slack_list_channels','slack_get_channel_history','slack_post_message','slack_reply_to_thread','slack_add_reaction','slack_get_users','slack_get_user_profile','slack_list_members','slack_search_messages','slack_create_channel','slack_set_channel_topic','slack_get_thread_replies','slack_list_workspaces','slack_upload_file','slack_delete_message','slack_schedule_message','...'] },
  { name: 'Supabase',         count: 30,  tools: ['supabase_execute_sql','supabase_apply_migration','supabase_list_migrations','supabase_list_tables','supabase_list_projects','supabase_get_project','supabase_create_project','supabase_pause_project','supabase_restore_project','supabase_list_organizations','supabase_get_organization','supabase_list_branches','supabase_create_branch','supabase_delete_branch','supabase_merge_branch','supabase_reset_branch','supabase_rebase_branch','supabase_deploy_edge_function','supabase_list_edge_functions','supabase_get_edge_function','supabase_get_logs','supabase_generate_typescript_types','supabase_list_extensions','supabase_get_advisors','supabase_get_cost','supabase_confirm_cost','supabase_get_project_url','supabase_get_publishable_keys','supabase_list_storage_buckets','supabase_get_storage_config'] },
  { name: 'Sentry',           count: 27,  tools: ['sentry_find_organizations','sentry_find_projects','sentry_list_issues','sentry_get_sentry_resource','sentry_list_events','sentry_list_issue_events','sentry_find_releases','sentry_find_teams','sentry_whoami','sentry_create_team','sentry_create_project','sentry_update_project','sentry_find_dsns','sentry_create_dsn','sentry_update_issue','sentry_get_issue_tag_values','sentry_analyze_issue_with_seer','sentry_get_profile_details','sentry_get_event_attachment','sentry_get_doc','sentry_search_docs','sentry_search_issues','sentry_search_events','sentry_search_issue_events','sentry_get_issue_details','sentry_get_trace_details','sentry_search_docs'] },
  { name: 'Cloudflare',       count: 89,  tools: ['worker_list','worker_get','worker_put','worker_delete','kv_namespace_list','kv_namespace_create','kv_key_list','kv_key_get','kv_key_put','kv_key_delete','r2_bucket_list','r2_bucket_create','r2_object_list','r2_object_get','d1_database_list','d1_database_create','d1_database_query','zone_list','dns_record_list','dns_record_create','dns_record_delete','pages_project_list','pages_project_get','pages_deployment_list','analytics_get','...'] },
  { name: 'Stripe',           count: 27,  tools: ['stripe_get_balance','stripe_list_customers','stripe_get_customer','stripe_create_customer','stripe_update_customer','stripe_search_customers','stripe_list_payment_intents','stripe_get_payment_intent','stripe_list_subscriptions','stripe_get_subscription','stripe_create_subscription','stripe_list_invoices','stripe_get_invoice','stripe_list_products','stripe_create_product','stripe_list_prices','stripe_create_price','stripe_list_events','stripe_get_event','stripe_list_webhooks','stripe_get_webhook','stripe_create_webhook','stripe_delete_webhook','stripe_list_payouts','stripe_get_payout','stripe_create_payment_link','stripe_create_checkout_session'] },
  { name: 'HubSpot',          count: 26,  tools: ['hubspot_list_contacts','hubspot_get_contact','hubspot_create_contact','hubspot_update_contact','hubspot_search_contacts','hubspot_delete_contact','hubspot_list_companies','hubspot_get_company','hubspot_create_company','hubspot_update_company','hubspot_search_companies','hubspot_list_deals','hubspot_get_deal','hubspot_create_deal','hubspot_update_deal','hubspot_move_deal_stage','hubspot_search_deals','hubspot_list_tickets','hubspot_get_ticket','hubspot_create_ticket','hubspot_update_ticket','hubspot_create_note','hubspot_create_task','hubspot_log_call','hubspot_list_pipelines','hubspot_list_pipeline_stages'] },
  { name: 'PostHog',          count: 18,  tools: ['posthog_capture_event','posthog_query_events','posthog_get_event_definitions','posthog_list_persons','posthog_get_person','posthog_search_persons','posthog_delete_person','posthog_list_feature_flags','posthog_get_feature_flag','posthog_create_feature_flag','posthog_update_feature_flag','posthog_toggle_feature_flag','posthog_list_insights','posthog_get_insight','posthog_create_trend_insight','posthog_list_cohorts','posthog_list_dashboards','posthog_list_projects'] },
  { name: 'Discord',          count: 28,  tools: ['discord_send_message','discord_edit_message','discord_delete_message','discord_get_message','discord_list_messages','discord_list_channels','discord_get_channel','discord_create_channel','discord_edit_channel','discord_delete_channel','discord_list_members','discord_get_member','discord_kick_member','discord_list_roles','discord_create_role','discord_add_role_to_member','discord_remove_role_from_member','discord_list_webhooks','discord_create_webhook','discord_send_webhook_message','discord_create_thread','discord_list_active_threads','discord_get_guild','discord_get_onboarding','discord_update_onboarding','discord_get_welcome_screen','discord_update_welcome_screen','discord_get_member_verification'] },
  { name: 'Vercel',           count: 15,  tools: ['vercel_list_projects','vercel_get_project','vercel_list_deployments','vercel_get_deployment','vercel_list_domains','vercel_list_env','vercel_cancel_deployment','vercel_list_teams','vercel_check_alias','vercel_get_build_logs','vercel_redeploy','vercel_delete_project','vercel_add_domain_to_project','vercel_remove_domain_from_project','vercel_get_project_members'] },
  { name: 'Railway',          count: 14,  tools: ['railway_list_projects','railway_get_project','railway_list_services','railway_list_deployments','railway_get_logs','railway_get_variables','railway_trigger_deploy','railway_list_environments','railway_get_service','railway_build_logs','railway_restart_deployment','railway_create_project','railway_delete_project','railway_get_usage'] },
  { name: 'Render',           count: 14,  tools: ['render_list_services','render_get_service','render_list_deploys','render_get_deploy','render_trigger_deploy','render_list_env_vars','render_put_env_var','render_suspend_service','render_resume_service','render_get_logs','render_list_custom_domains','render_add_custom_domain','render_delete_custom_domain','render_scale_service'] },
  { name: 'Twilio',           count: 16,  tools: ['twilio_send_sms','twilio_send_whatsapp','twilio_list_messages','twilio_get_message','twilio_create_call','twilio_list_calls','twilio_get_call','twilio_list_numbers','twilio_search_available_numbers','twilio_purchase_number','twilio_release_number','twilio_create_verify_service','twilio_list_verify_services','twilio_send_verification','twilio_check_verification','twilio_lookup_phone_number'] },
  { name: 'Resend',           count: 14,  tools: ['resend_send_email','resend_get_email','resend_cancel_email','resend_list_domains','resend_get_domain','resend_create_domain','resend_verify_domain','resend_delete_domain','resend_list_api_keys','resend_create_api_key','resend_delete_api_key','resend_list_audiences','resend_create_audience','resend_delete_audience'] },
  { name: 'Figma',            count: 15,  tools: ['figma_get_file','figma_get_file_nodes','figma_list_file_versions','figma_get_comments','figma_post_comment','figma_delete_comment','figma_get_team_components','figma_get_component','figma_get_component_sets','figma_get_team_projects','figma_get_project_files','figma_get_local_variables','figma_get_published_variables','figma_export_images','figma_list_webhooks'] },
  { name: 'Google Calendar',  count: 12,  tools: ['gcal_upcoming','gcal_today','gcal_this_week','gcal_search','gcal_list_calendars','gcal_create_event','gcal_delete_event','gcal_update_event','gcal_get_event','gcal_get_free_busy','gcal_add_attendee','gcal_list_upcoming_for_calendar'] },
  { name: 'Airtable',         count: 5,   tools: ['search_airtable','airtable_list_bases','airtable_list_tables','airtable_get_records','airtable_create_record'] },
  { name: 'Trello',           count: 4,   tools: ['search_trello','trello_list_boards','trello_list_cards','trello_create_card'] },
  { name: 'Todoist',          count: 4,   tools: ['search_todoist','todoist_list_tasks','todoist_create_task','todoist_complete_task'] },
  { name: 'n8n',              count: 22,  tools: ['n8n_list_workflows','n8n_get_workflow','n8n_activate_workflow','n8n_deactivate_workflow','n8n_execute_workflow','n8n_delete_workflow','n8n_list_executions','n8n_get_execution','n8n_stop_execution','n8n_list_tags','n8n_get_workflow_runs','n8n_trigger_webhook','n8n_create_workflow','n8n_update_workflow','n8n_list_credentials','n8n_create_credential','n8n_delete_credential','n8n_list_variables','n8n_create_variable','n8n_delete_variable','n8n_create_tag','n8n_delete_tag'] },
  { name: 'Shopify',          count: 31,  tools: ['shopify_get_shop','shopify_list_orders','shopify_get_order','shopify_cancel_order','shopify_fulfill_order','shopify_list_products','shopify_get_product','shopify_create_product','shopify_update_product','shopify_delete_product','shopify_list_customers','shopify_get_customer','shopify_search_customers','shopify_create_customer','shopify_update_customer','shopify_list_collections','shopify_get_inventory_levels','shopify_list_webhooks','shopify_create_webhook','shopify_list_price_rules','shopify_list_locations','shopify_update_inventory','shopify_list_product_variants','shopify_update_product_variant','shopify_refund_order','shopify_list_draft_orders','shopify_create_draft_order','shopify_complete_draft_order','shopify_list_transactions','shopify_create_collection','shopify_list_smart_collections'] },
]

const TOTAL_TOOLS = CONNECTORS.reduce((s, c) => s + c.count, 0)

const NAV_SECTIONS = [
  {
    label: 'Démarrage',
    items: [
      { id: 'installation',   label: 'Installation' },
      { id: 'lancer',         label: 'Premier démarrage' },
    ],
  },
  {
    label: 'Clients IA MCP',
    items: [
      { id: 'claude-desktop', label: 'Configurer un client' },
    ],
  },
  {
    label: 'Concepts',
    items: [
      { id: 'mcp',            label: 'C\'est quoi MCP ?' },
      { id: 'privacy',        label: 'Confidentialité' },
      { id: 'tools',          label: 'Les connecteurs' },
      { id: 'stack',          label: 'Stack technique' },
    ],
  },
  {
    label: 'Features à venir',
    items: [
      { id: 'p2p',            label: 'Réseau P2P' },
    ],
  },
]

// ─── Connectors Table ─────────────────────────────────────────────────────────

function ConnectorsTable() {
  const [open, setOpen] = useState<string | null>(null)

  return (
    <ToolTable>
      <thead>
        <tr>
          <Th style={{ width: 32 }}></Th>
          <Th>Connecteur</Th>
          <Th style={{ width: 120 }}>Tools</Th>
        </tr>
      </thead>
      <tbody>
        {CONNECTORS.map(c => (
          <React.Fragment key={c.name}>
            <ConnectorRow
              $open={open === c.name}
              onClick={() => setOpen(open === c.name ? null : c.name)}
            >
              <Td style={{ width: 32, paddingRight: 0 }}>
                <Chevron $open={open === c.name}>▶</Chevron>
              </Td>
              <Td style={{ color: '#e8eaf0', fontWeight: 600 }}>{c.name}</Td>
              <Td><CountBadge>{c.count}</CountBadge></Td>
            </ConnectorRow>
            {open === c.name && (
              <ConnectorExpand>
                <ToolsGrid colSpan={3}>
                  {c.tools.map(t => (
                    <ToolPill key={t}>{t}</ToolPill>
                  ))}
                </ToolsGrid>
              </ConnectorExpand>
            )}
          </React.Fragment>
        ))}
      </tbody>
    </ToolTable>
  )
}

// ─── Page ─────────────────────────────────────────────────────────────────────

export default function DocsPage() {
  const [activeId, setActiveId] = useState('installation')
  const [activeClient, setActiveClient] = useState<McpClient>('Claude Desktop')
  // Scrollspy — picks the section whose top is closest to (but above) 120px from viewport top
  useEffect(() => {
    const allIds = NAV_SECTIONS.flatMap(s => s.items.map(i => i.id))

    const onScroll = () => {
      const OFFSET = 120
      let best: string | null = null
      let bestDist = Infinity

      allIds.forEach(id => {
        const el = document.getElementById(id)
        if (!el) return
        const rect = el.getBoundingClientRect()
        // distance from top of viewport, only sections that have scrolled past OFFSET
        if (rect.top <= OFFSET) {
          const dist = OFFSET - rect.top
          if (dist < bestDist) { bestDist = dist; best = id }
        }
      })

      if (best) setActiveId(best)
    }

    window.addEventListener('scroll', onScroll, { passive: true })
    onScroll() // run on mount
    return () => window.removeEventListener('scroll', onScroll)
  }, [])

  const scrollTo = (id: string) => {
    document.getElementById(id)?.scrollIntoView({ behavior: 'smooth', block: 'start' })
  }

  const clientCfg = MCP_CONFIGS[activeClient]

  return (
    <>
      <GlobalStyle />
      <Shell>
        {/* Top bar */}
        <TopBar>
          <TopBarLogo href="/">
            <SiteLogo size={26} />
            OSMOzzz
          </TopBarLogo>
          <TopBarSep>/</TopBarSep>
          <TopBarLabel>Documentation</TopBarLabel>
          <TopBarRight>
            <GhLink href="https://github.com/platre11/OSMOzzz" target="_blank" rel="noreferrer">
              <GithubIcon />
              GitHub
            </GhLink>
          </TopBarRight>
        </TopBar>

        <Body>
          {/* Sidebar */}
          <Sidebar>
            <SideLogoBlock>
              <SiteLogo size={22} />
              <SideLogoName>OSMOzzz</SideLogoName>
            </SideLogoBlock>

            {NAV_SECTIONS.map((group, i) => (
              <SideGroup key={group.label}>
                {i > 0 && <SideGroupSep />}
                <SideGroupLabel>{group.label}</SideGroupLabel>
                {group.items.map(item => (
                  <SideItem
                    key={item.id}
                    $active={activeId === item.id}
                    onClick={() => scrollTo(item.id)}
                  >
                    {item.label}
                  </SideItem>
                ))}
              </SideGroup>
            ))}
          </Sidebar>

          {/* Main content */}
          <Content>

            {/* ── DÉMARRAGE ──────────────────────────────────────────────── */}
            <DocH1>Documentation OSMOzzz</DocH1>
            <DocP>
              OSMOzzz connecte votre client IA à toutes vos données — emails, fichiers,
              notes, calendrier, outils cloud — 100 % en local. Rien ne quitte votre machine.
            </DocP>

            <DocDivider />

            <DocSection id="installation">
              <DocH2>Installation</DocH2>
              <DocP>
                Téléchargez le fichier <InlineCode>.pkg</InlineCode> et double-cliquez dessus.
                L'installeur place le binaire dans <InlineCode>/usr/local/bin/osmozzz</InlineCode>.
              </DocP>
              <StepList>
                <StepItem>
                  <StepBody>
                    <strong>Téléchargez</strong> la dernière version depuis la page d'accueil ou GitHub Releases.
                  </StepBody>
                </StepItem>
                <StepItem>
                  <StepBody>
                    <strong>Double-cliquez</strong> sur le fichier <InlineCode>osmozzz.pkg</InlineCode> et suivez l'installeur.
                  </StepBody>
                </StepItem>
                <StepItem>
                  <StepBody>
                    C'est tout. Le binaire est installé dans <InlineCode>/usr/local/bin/osmozzz</InlineCode>.
                  </StepBody>
                </StepItem>
              </StepList>
            </DocSection>

            <DocSection id="lancer">
              <DocH2>Lancer OSMOzzz</DocH2>
              <DocP>
                C'est automatique. Le script d'installation enregistre OSMOzzz comme
                service système — il démarre au login et tourne en arrière-plan sans
                aucune intervention. Le dashboard s'ouvre dans votre navigateur dès
                la fin de l'installation.
              </DocP>
              <DocP>
                Accédez au dashboard à tout moment sur{' '}
                <InlineCode>http://localhost:7878</InlineCode>.
                C'est depuis là que vous configurez vos connecteurs (Gmail, GitHub, Notion, Jira…).
              </DocP>
            </DocSection>

            <DocDivider />

            {/* ── CLIENTS IA MCP ─────────────────────────────────────────── */}
            <DocH1>Clients IA compatibles MCP</DocH1>
            <DocP>
              MCP (Model Context Protocol) est un protocole ouvert. OSMOzzz fonctionne
              avec tous les clients IA qui le supportent. Sélectionnez votre client pour
              obtenir la configuration exacte.
            </DocP>

            {/* Tabs */}
            <TabsBar>
              {MCP_CLIENTS.map(c => (
                <Tab key={c} $active={activeClient === c} onClick={() => setActiveClient(c)}>
                  {c}
                </Tab>
              ))}
            </TabsBar>

            <DocSection id="claude-desktop" style={{ display: activeClient === 'Claude Desktop' ? 'block' : 'none' }}>
              <DocH2>Claude Desktop</DocH2>
              <DocP>
                Ouvrez ou créez le fichier de configuration de Claude Desktop, ajoutez
                le bloc <InlineCode>mcpServers</InlineCode> et relancez l'application.
              </DocP>
              <CodeBlock id="cfg-claude" lang="json" file={clientCfg.file} code={MCP_CONFIGS['Claude Desktop'].code} />
            </DocSection>

            <DocSection id="cursor" style={{ display: activeClient === 'Cursor' ? 'block' : 'none' }}>
              <DocH2>Cursor</DocH2>
              <DocP>
                Créez ou modifiez le fichier <InlineCode>~/.cursor/mcp.json</InlineCode>,
                ajoutez le bloc ci-dessous et redémarrez Cursor.
              </DocP>
              <CodeBlock id="cfg-cursor" lang="json" file={MCP_CONFIGS['Cursor'].file} code={MCP_CONFIGS['Cursor'].code} />
            </DocSection>

            <DocSection id="windsurf" style={{ display: activeClient === 'Windsurf' ? 'block' : 'none' }}>
              <DocH2>Windsurf</DocH2>
              <DocP>
                Modifiez le fichier de config MCP de Windsurf et redémarrez l'éditeur.
              </DocP>
              <CodeBlock id="cfg-windsurf" lang="json" file={MCP_CONFIGS['Windsurf'].file} code={MCP_CONFIGS['Windsurf'].code} />
            </DocSection>

            <DocSection id="zed" style={{ display: activeClient === 'Zed' ? 'block' : 'none' }}>
              <DocH2>Zed</DocH2>
              <DocP>
                Zed utilise une clé <InlineCode>context_servers</InlineCode> dans ses settings.
                Ajoutez le bloc ci-dessous dans <InlineCode>~/.config/zed/settings.json</InlineCode>.
              </DocP>
              <CodeBlock id="cfg-zed" lang="json" file={MCP_CONFIGS['Zed'].file} code={MCP_CONFIGS['Zed'].code} />
            </DocSection>

            <DocDivider />

            {/* ── CONCEPTS ───────────────────────────────────────────────── */}
            <DocH1>Concepts</DocH1>

            <DocSection id="mcp">
              <DocH2>C'est quoi MCP ?</DocH2>
              <DocP>
                Le <strong style={{ color: '#e8eaf0' }}>Model Context Protocol</strong> est un standard ouvert
                créé par Anthropic en 2024. Il définit comment un client IA (Claude, Cursor, Zed…)
                peut appeler des outils externes — appelés <strong style={{ color: '#e8eaf0' }}>tools MCP</strong> — pour
                accéder à des données ou déclencher des actions.
              </DocP>
              <DocP>
                OSMOzzz agit comme un <strong style={{ color: '#e8eaf0' }}>pare-feu MCP</strong>.
                Il se place entre votre client IA et vos outils cloud — Notion, GitHub, Slack, Gmail,
                Linear, Jira… Votre client IA ne se connecte jamais directement à ces services :
                il passe par OSMOzzz, qui centralise les accès, filtre les données sensibles
                et contrôle ce que l'IA peut voir ou faire.
              </DocP>
            </DocSection>

            <DocSection id="privacy">
              <DocH2>Confidentialité & contrôle</DocH2>
              <DocP>
                OSMOzzz intègre quatre systèmes de sécurité indépendants, accessibles depuis le dashboard → Actions MCP.
              </DocP>

              <DocH3>Flux d'actions</DocH3>
              <DocP>
                Chaque appel tool de votre client IA est enregistré en temps réel : connecteur utilisé,
                requête exécutée, résultats retournés, et toutes les transformations de sécurité appliquées.
                Vous savez exactement ce que votre IA a fait et ce qu'elle a vu, à chaque instant.
                Vous pouvez également valider manuellement certaines actions avant exécution,
                et consulter l'historique complet des appels passés.
              </DocP>

              <DocH3>Sources</DocH3>
              <DocP>
                Contrôlez l'accès de votre client IA connecteur par connecteur : activez ou désactivez
                l'accès en un clic, et choisissez si les tools s'exécutent automatiquement ou nécessitent
                votre validation manuelle avant d'être exécutés.
              </DocP>

              <DocH3>Confidentialité</DocH3>
              <DocP>
                Masquez automatiquement les adresses email et numéros de téléphone dans toutes les
                réponses transmises à votre IA. Configurez des alias d'identité pour remplacer
                les vrais noms par des pseudonymes — votre IA travaille avec "Collaborateur-A"
                plutôt qu'avec "Jean Dupont".
              </DocP>

              <DocH3>Bases de données</DocH3>
              <DocP>
                Définissez la visibilité de chaque colonne de vos bases SQL colonne par colonne :
                valeur brute transmise à l'IA, token opaque stable (l'IA peut raisonner dessus
                sans jamais voir la vraie valeur), ou colonne entièrement bloquée.
              </DocP>
            </DocSection>

            <DocSection id="tools">
              <DocH2>Les {TOTAL_TOOLS}+ tools MCP</DocH2>
              <DocP>
                OSMOzzz expose {TOTAL_TOOLS}+ tools à votre client IA via {CONNECTORS.length} connecteurs cloud.
                Cliquez sur un connecteur pour voir la liste de ses tools.
              </DocP>
              <ConnectorsTable />
            </DocSection>

            {/* ── STACK ──────────────────────────────────────────────────── */}
            <DocSection id="stack">
              <DocH2>Stack technique</DocH2>
              <DocP>
                OSMOzzz est entièrement écrit en <strong style={{ color: '#e8eaf0' }}>Rust</strong> —
                un binaire unique sans runtime, sans Node.js, sans Python.
                Le dashboard React est embarqué directement dans le binaire.
              </DocP>
              <ToolTable style={{ marginTop: 16 }}>
                <thead>
                  <tr>
                    <Th>Composant</Th>
                    <Th>Technologie</Th>
                  </tr>
                </thead>
                <tbody>
                  {[
                    ['Langage', 'Rust 2021'],
                    ['Runtime async', 'Tokio'],
                    ['API REST & dashboard', 'Axum'],
                    ['Dashboard UI', 'React 18 + TypeScript + Vite'],
                    ['Transport MCP', 'JSON-RPC 2.0 stdin/stdout'],
                    ['Connecteurs natifs', 'reqwest (HTTP direct)'],
                    ['Proxies MCP', 'Bun / bunx (subprocesses npm)'],
                    ['Stockage config', 'TOML (~/.osmozzz/*.toml)'],
                  ].map(([comp, tech]) => (
                    <tr key={comp}>
                      <Td style={{ color: '#e8eaf0', fontWeight: 600 }}>{comp}</Td>
                      <Td><InlineCode>{tech}</InlineCode></Td>
                    </tr>
                  ))}
                </tbody>
              </ToolTable>
            </DocSection>

            <DocDivider />

            {/* ── FEATURES À VENIR ───────────────────────────────────────── */}
            <DocH1>Features à venir</DocH1>

            <DocSection id="p2p">
              <DocH2>Réseau P2P mesh</DocH2>
              <DocP>
                La prochaine grande fonctionnalité d'OSMOzzz : connecter plusieurs instances entre elles
                en pair-à-pair. Votre IA pourra interroger les outils d'un collègue — ses emails,
                son Linear, son Supabase — avec des permissions granulaires définies par chaque pair.
              </DocP>
              <DocP>
                Chaque machine aura une identité cryptographique unique (Ed25519). Toutes les données
                sortantes passeront par les filtres de confidentialité du propriétaire avant d'être
                transmises. Le réseau est chiffré de bout en bout via{' '}
                <strong style={{ color: '#e8eaf0' }}>iroh (QUIC)</strong>.
              </DocP>
              <StepList>
                <StepItem>
                  <StepBody>
                    <strong>Permissions granulaires</strong> — chaque pair choisit connecteur par connecteur ce qu'il partage : automatique, validation manuelle, ou bloqué.
                  </StepBody>
                </StepItem>
                <StepItem>
                  <StepBody>
                    <strong>Confidentialité bout en bout</strong> — alias d'identité et filtre de confidentialité appliqués sur toutes les données sortantes avant envoi au pair.
                  </StepBody>
                </StepItem>
                <StepItem>
                  <StepBody>
                    <strong>Zéro configuration réseau</strong> — connexion via relay chiffré, fonctionne derrière NAT et firewall sans ouvrir de port.
                  </StepBody>
                </StepItem>
              </StepList>
            </DocSection>

          </Content>
        </Body>
      </Shell>
    </>
  )
}
