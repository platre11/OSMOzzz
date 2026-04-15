import axios from 'axios'

const BASE = '/api'

export interface SourceStatus {
  count: number
  last_sync: string | null
  error: string | null
}

export interface PerfMetrics {
  db_disk_mb: number
  process_rss_mb: number | null
  total_vectors: number
  estimated_ram_mb: number
}

export interface StatusData {
  daemon_status: string
  sources: Record<string, SourceStatus>
  perf: PerfMetrics
}

export interface SearchDoc {
  url: string
  title: string | null
  content: string
  date: string | null
}

export interface SourceGroup {
  source: string
  results: SearchDoc[]
}

export interface GroupedSearchResponse {
  groups: SourceGroup[]
}

export interface RecentDoc {
  url: string
  title: string | null
  content: string
  source: string
  source_ts: number | null
}

export interface ConnectorStatus {
  configured: boolean
  display: string | null
}

export interface ConfigData {
  gmail:    ConnectorStatus
  notion:   ConnectorStatus
  github:   ConnectorStatus
  linear:   ConnectorStatus
  jira:     ConnectorStatus
  slack:    ConnectorStatus
  trello:   ConnectorStatus
  todoist:  ConnectorStatus
  gitlab:   ConnectorStatus
  airtable: ConnectorStatus
  obsidian: ConnectorStatus
  supabase: ConnectorStatus
  sentry:     ConnectorStatus
  cloudflare: ConnectorStatus
  vercel:  ConnectorStatus
  railway: ConnectorStatus
  render:  ConnectorStatus
  google:  ConnectorStatus
  stripe:  ConnectorStatus
  hubspot: ConnectorStatus
  posthog: ConnectorStatus
  resend:  ConnectorStatus
  discord:  ConnectorStatus
  twilio:   ConnectorStatus
  figma:    ConnectorStatus
  reddit:   ConnectorStatus
  calendly: ConnectorStatus
  n8n:      ConnectorStatus
  shopify:  ConnectorStatus
}

export interface ContactItem {
  phone: string
  last_message: string
  last_ts: number
  count: number
}

export interface MessageItem {
  ts: number
  is_me: boolean
  text: string
  date: string | null
}

export interface BlacklistEntry {
  kind: string
  source: string
  identifier: string
  title: string | null
  content: string | null
}

export interface BlacklistResponse {
  entries: BlacklistEntry[]
}

export interface MyIdentity {
  peer_id: string
  display_name: string
}

export interface PeerResponse {
  peer_id: string
  display_name: string
  addresses: string[]
  connected: boolean
  last_seen: number | null
  shared_sources: string[]
}

export interface PeerPermissions {
  allowed_sources: string[]
  max_results_per_query: number
}

/** Mode d'accès d'un peer à un connecteur/tool spécifique */
export type ToolAccessMode = 'auto' | 'require' | 'disabled'

/** Map connecteur → mode (ex: { github: 'auto', linear: 'require', stripe: 'disabled' }) */
export type ToolPermissions = Record<string, ToolAccessMode>

export interface QueryHistoryEntry {
  ts: number
  peer_id: string
  peer_name: string
  /** Pour "search" : la query texte. Pour "tool_call" : le nom du tool. */
  query: string
  results_count: number
  blocked: boolean
  /** "search" | "tool_call" — les anciennes entrées sont "search" par défaut */
  kind: string
  /** Contenu brut du résultat (tronqué à 4 Ko) — undefined si bloqué ou vide */
  data?: string
}

export const ALL_SOURCES = [
  'file', 'notion', 'github', 'linear', 'jira',
  'slack', 'trello', 'todoist', 'gitlab', 'airtable', 'obsidian',
  'email', 'imessage', 'terminal', 'chrome', 'safari', 'notes', 'calendar',
] as const

// ─── Actions orchestrateur ───────────────────────────────────────────────────

export type ActionStatus = 'pending' | 'approved' | 'rejected' | 'expired'

export interface ActionRequest {
  id: string
  tool: string
  params: Record<string, unknown>
  preview: string
  status: ActionStatus
  created_at: number
  expires_at: number
  execution_result?: string
}

export interface ActionEvent {
  kind: 'new' | 'updated'
  action: ActionRequest
}

// ─── Confidentialité ─────────────────────────────────────────────────────────

export interface PrivacyConfig {
  email: boolean
  phone: boolean
}

export type ColumnRule = 'free' | 'tokenize' | 'block'

export interface DbColumnSchema {
  column_name: string
  data_type: string
  ordinal_position?: number
}

export interface DbTableSchema {
  table_name: string
  columns: DbColumnSchema[]
}

export interface ProjectSecurityConfig {
  supabase: Record<string, Record<string, ColumnRule>>
  column_order?: Record<string, string[]>
}

export interface DbSecurityConfig {
  active_project_id?: string
  supabase: Record<string, Record<string, ColumnRule>>
  column_order?: Record<string, string[]>
  projects?: Record<string, ProjectSecurityConfig>
}

export const api = {
  getStatus: async (): Promise<StatusData> => {
    const r = await axios.get(`${BASE}/status`)
    return r.data.data
  },

  search: async (q: string, filters?: { source?: string; from?: string; to?: string }): Promise<GroupedSearchResponse> => {
    const r = await axios.get(`${BASE}/search`, { params: { q, ...filters } })
    return r.data.data ?? { groups: [] }
  },

  getRecent: async (source: string, limit = 20, offset = 0, filters?: { q?: string; from?: string; to?: string }): Promise<RecentDoc[]> => {
    const r = await axios.get(`${BASE}/recent`, { params: { source, limit, offset, ...filters } })
    return r.data.data ?? []
  },

  getConfig: async (): Promise<ConfigData> => {
    const r = await axios.get(`${BASE}/config`)
    return r.data.data
  },

  saveGmail: async (username: string, app_password: string) => {
    await axios.post(`${BASE}/config/gmail`, { username, app_password })
  },

  saveNotion: async (token: string) => {
    await axios.post(`${BASE}/config/notion`, { token })
  },

  saveGithub: async (token: string, repos: string) => {
    await axios.post(`${BASE}/config/github`, { token, repos })
  },

  saveLinear: async (api_key: string) => {
    await axios.post(`${BASE}/config/linear`, { api_key })
  },

  saveJira: async (base_url: string, email: string, token: string) => {
    await axios.post(`${BASE}/config/jira`, { base_url, email, token })
  },

  saveSlack: async (token: string, team_id: string, channels: string) => {
    await axios.post(`${BASE}/config/slack`, { token, team_id, channels })
  },

  saveTrello: async (api_key: string, token: string) => {
    await axios.post(`${BASE}/config/trello`, { api_key, token })
  },

  saveTodoist: async (token: string) => {
    await axios.post(`${BASE}/config/todoist`, { token })
  },

  saveGitlab: async (token: string, base_url: string, groups: string) => {
    await axios.post(`${BASE}/config/gitlab`, { token, base_url, groups })
  },

  saveAirtable: async (token: string, bases: string) => {
    await axios.post(`${BASE}/config/airtable`, { token, bases })
  },

  saveObsidian: async (vault_path: string) => {
    await axios.post(`${BASE}/config/obsidian`, { vault_path })
  },

  saveSupabase: async (access_token: string, project_id?: string) => {
    await axios.post(`${BASE}/config/supabase`, { access_token, project_id })
  },

  saveSentry: async (token: string, host?: string) => {
    await axios.post(`${BASE}/config/sentry`, { token, host: host ?? '' })
  },

  saveCloudflare: async (api_token: string, account_id: string) => {
    await axios.post(`${BASE}/config/cloudflare`, { api_token, account_id })
  },

  saveVercel: async (token: string, team_id?: string) => {
    await axios.post(`${BASE}/config/vercel`, { token, team_id: team_id ?? '' })
  },

  saveRailway: async (token: string) => {
    await axios.post(`${BASE}/config/railway`, { token })
  },

  saveRender: async (token: string) => {
    await axios.post(`${BASE}/config/render`, { token })
  },

  saveGoogle: async (username: string, app_password: string) => {
    await axios.post(`${BASE}/config/google`, { username, app_password })
  },

  saveStripe: async (secret_key: string) => {
    await axios.post(`${BASE}/config/stripe`, { secret_key })
  },

  saveHubspot: async (token: string) => {
    await axios.post(`${BASE}/config/hubspot`, { token })
  },

  savePosthog: async (api_key: string, project_id: string, host?: string) => {
    await axios.post(`${BASE}/config/posthog`, { api_key, project_id, host: host ?? '' })
  },

  saveResend: async (api_key: string) => {
    await axios.post(`${BASE}/config/resend`, { api_key })
  },

  saveDiscord: async (bot_token: string, guild_id?: string) => {
    await axios.post(`${BASE}/config/discord`, { bot_token, guild_id: guild_id ?? '' })
  },

  saveTwilio: async (account_sid: string, auth_token: string, from_number?: string) => {
    await axios.post(`${BASE}/config/twilio`, { account_sid, auth_token, from_number: from_number ?? '' })
  },

  saveFigma: async (token: string, team_id?: string) => {
    await axios.post(`${BASE}/config/figma`, { token, team_id: team_id ?? '' })
  },

  saveReddit: async (client_id: string, client_secret: string, username: string, password: string) => {
    await axios.post(`${BASE}/config/reddit`, { client_id, client_secret, username, password })
  },

  saveCalendly: async (token: string) => {
    await axios.post(`${BASE}/config/calendly`, { token })
  },

  saveN8n: async (api_url: string, api_key: string) => {
    await axios.post(`${BASE}/config/n8n`, { api_url, api_key })
  },

  saveShopify: async (shop_domain: string, access_token: string) => {
    await axios.post(`${BASE}/config/shopify`, { shop_domain, access_token })
  },

  open: async (url: string): Promise<void> => {
    await axios.get(`${BASE}/open`, { params: { url } })
  },

  getImessageContacts: async (): Promise<ContactItem[]> => {
    const r = await axios.get(`${BASE}/messages/contacts`)
    return r.data.data ?? []
  },

  getImessageConversation: async (phone: string, limit = 200): Promise<MessageItem[]> => {
    const r = await axios.get(`${BASE}/messages/conversation`, { params: { phone, limit } })
    return r.data.data ?? []
  },

  banUrl: async (url: string): Promise<void> => {
    await axios.post(`${BASE}/ban`, { kind: 'url', url })
  },

  banSourceItem: async (source: string, identifier: string): Promise<void> => {
    await axios.post(`${BASE}/ban`, { kind: 'source', source, identifier })
  },

  getBlacklist: async (): Promise<BlacklistResponse> => {
    const r = await axios.get(`${BASE}/blacklist`)
    return r.data.data ?? { entries: [] }
  },

  unbanUrl: async (url: string): Promise<void> => {
    await axios.post(`${BASE}/unban`, { kind: 'url', url })
  },

  unbanSourceItem: async (source: string, identifier: string): Promise<void> => {
    await axios.post(`${BASE}/unban`, { kind: 'source', source, identifier })
  },

  compact: async (): Promise<void> => {
    await axios.post(`${BASE}/compact`)
  },

  reindexImessage: async (): Promise<string> => {
    const r = await axios.post(`${BASE}/reindex/imessage`)
    return r.data.data ?? ''
  },

  indexPreview: async (): Promise<Record<string, number>> => {
    const r = await axios.get(`${BASE}/index/preview`)
    return r.data.data?.extensions ?? {}
  },

  indexProgress: async (): Promise<{
    running: boolean; total: number; processed: number;
    indexed: number; skipped: number; current_file: string
  }> => {
    const r = await axios.get(`${BASE}/index/progress`)
    return r.data.data ?? { running: false, total: 0, processed: 0, indexed: 0, skipped: 0, current_file: '' }
  },

  indexFiles: async (extensions?: string[], path?: string): Promise<void> => {
    await axios.post(`${BASE}/index`, { path, extensions })
  },

  filesSearch: async (q: string, exts?: string): Promise<Array<{
    path: string; name: string; ext: string; size_kb: number; snippet: string
  }>> => {
    const r = await axios.get(`${BASE}/files/search`, { params: { q, exts: exts ?? '', limit: 40 } })
    return r.data.data ?? []
  },

  // ─── Réseau P2P ─────────────────────────────────────────────────────────────

  getNetworkPeers: async (): Promise<PeerResponse[]> => {
    const r = await axios.get(`${BASE}/network/peers`)
    return r.data.data ?? []
  },

  generateInvite: async (): Promise<{ link: string; peer_id: string }> => {
    const r = await axios.post(`${BASE}/network/invite`)
    return r.data.data
  },

  connectPeer: async (link: string, display_name: string): Promise<void> => {
    await axios.post(`${BASE}/network/connect`, { link, display_name })
  },

  deletePeer: async (peer_id: string): Promise<void> => {
    await axios.delete(`${BASE}/network/peers/${peer_id}`)
  },

  getPeerPermissions: async (peer_id: string): Promise<PeerPermissions> => {
    const r = await axios.get(`${BASE}/network/permissions/${peer_id}`)
    return r.data.data
  },

  setPeerPermissions: async (peer_id: string, perms: PeerPermissions): Promise<void> => {
    await axios.post(`${BASE}/network/permissions/${peer_id}`, perms)
  },

  getNetworkHistory: async (): Promise<QueryHistoryEntry[]> => {
    const r = await axios.get(`${BASE}/network/history`)
    return r.data.data ?? []
  },

  getMyIdentity: async (): Promise<MyIdentity> => {
    const r = await axios.get(`${BASE}/network/identity`)
    return r.data.data
  },

  resyncPermissions: async (): Promise<void> => {
    await axios.post(`${BASE}/network/resync`)
  },

  getPeerGrantedPermissions: async (peer_id: string): Promise<ToolPermissions> => {
    const r = await axios.get(`${BASE}/network/granted-permissions/${peer_id}`)
    const data = r.data.data
    if (!data) return {}
    // Retourne les tool_permissions (ce que le peer nous autorise)
    return data.tool_permissions ?? {}
  },

  getPeerToolPermissions: async (peer_id: string): Promise<ToolPermissions> => {
    const r = await axios.get(`${BASE}/network/tool-permissions/${peer_id}`)
    return r.data.data ?? {}
  },

  setPeerToolPermissions: async (peer_id: string, permissions: ToolPermissions): Promise<void> => {
    await axios.post(`${BASE}/network/tool-permissions/${peer_id}`, { permissions })
  },

  getConfiguredConnectors: async (): Promise<string[]> => {
    const r = await axios.get(`${BASE}/configured-connectors`)
    return r.data.data ?? []
  },

  // ─── IA locale ───────────────────────────────────────────────────────────────

  /**
   * Envoie une question au LLM local et reçoit une réponse en streaming (SSE).
   * Retourne un ReadableStreamDefaultReader — itérer avec reader.read() pour
   * recevoir les tokens au fur et à mesure.
   */
  chatStream: async (query: string): Promise<ReadableStreamDefaultReader<Uint8Array>> => {
    const response = await fetch(`${BASE}/chat`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ query }),
    })
    if (!response.body) throw new Error('Pas de stream SSE')
    return response.body.getReader()
  },

  // ─── Confidentialité ─────────────────────────────────────────────────────────

  getPrivacy: async (): Promise<PrivacyConfig> => {
    const r = await axios.get(`${BASE}/privacy`)
    return r.data.data ?? { email: false, phone: false }
  },

  setPrivacy: async (config: PrivacyConfig): Promise<void> => {
    await axios.post(`${BASE}/privacy`, config)
  },

  // ─── Actions P2P (flux réseau — séparé des actions locales Claude) ──────────

  getP2pPending: async (): Promise<ActionRequest[]> => {
    const r = await axios.get(`${BASE}/network/p2p-pending`)
    return r.data.data ?? []
  },

  approveP2pAction: async (id: string): Promise<ActionRequest> => {
    const r = await axios.post(`${BASE}/network/p2p-actions/${id}/approve`)
    return r.data.data
  },

  rejectP2pAction: async (id: string): Promise<ActionRequest> => {
    const r = await axios.post(`${BASE}/network/p2p-actions/${id}/reject`)
    return r.data.data
  },

  // ─── Actions orchestrateur ───────────────────────────────────────────────────

  getActionsPending: async (): Promise<ActionRequest[]> => {
    const r = await axios.get(`${BASE}/actions/pending`)
    return r.data.data ?? []
  },

  getActionsAll: async (): Promise<ActionRequest[]> => {
    const r = await axios.get(`${BASE}/actions`)
    return r.data.data ?? []
  },

  approveAction: async (id: string): Promise<ActionRequest> => {
    const r = await axios.post(`${BASE}/actions/${id}/approve`)
    return r.data.data
  },

  rejectAction: async (id: string): Promise<ActionRequest> => {
    const r = await axios.post(`${BASE}/actions/${id}/reject`)
    return r.data.data
  },

  getPermissions: async (): Promise<Record<string, boolean>> => {
    const r = await axios.get(`${BASE}/permissions`)
    return r.data.data ?? {}
  },

  savePermissions: async (perms: Record<string, boolean>): Promise<void> => {
    await axios.post(`${BASE}/permissions`, perms)
  },

  // ─── Accès sources MCP ───────────────────────────────────────────────────────

  getSourceAccess: async (): Promise<Record<string, boolean>> => {
    const r = await axios.get(`${BASE}/source-access`)
    return r.data.data ?? {}
  },

  getAudit: async (limit = 200): Promise<Array<{
    ts: number; tool: string; query: string; results: number; blocked: boolean; data?: string | Record<string, unknown>;
  }>> => {
    const r = await axios.get(`${BASE}/audit`, { params: { limit } })
    return r.data.data ?? []
  },

  // ─── Alias Engine ────────────────────────────────────────────────────────────

  getAliases: async (): Promise<{ aliases: Array<{ real: string; alias: string; alias_type?: string }>; types: string[] }> => {
    const r = await axios.get(`${BASE}/aliases`)
    const raw = r.data.data
    // Nouveau format : { aliases: [...], types: [...] }
    if (raw && !Array.isArray(raw) && Array.isArray(raw.aliases)) {
      return { aliases: raw.aliases ?? [], types: raw.types ?? [] }
    }
    // Ancien format : tableau plat [{real, alias}]
    return { aliases: Array.isArray(raw) ? raw : [], types: [] }
  },

  saveAliases: async (aliases: Array<{ real: string; alias: string; alias_type?: string }>, types: string[]): Promise<void> => {
    await axios.post(`${BASE}/aliases`, { aliases, types })
  },

  saveSourceAccess: async (access: Record<string, boolean>): Promise<void> => {
    await axios.post(`${BASE}/source-access`, access)
  },

  // ─── Sécurité base de données ─────────────────────────────────────────────

  getSupabaseProjects: async (): Promise<Array<{ id: string; name: string; region: string }>> => {
    const r = await axios.get(`${BASE}/db/supabase/projects`)
    if (!r.data.ok) throw new Error(r.data.error ?? 'Erreur inconnue')
    return Array.isArray(r.data.data) ? r.data.data : []
  },

  saveSupabaseProject: async (project_id: string): Promise<void> => {
    const r = await axios.post(`${BASE}/db/supabase/project`, { project_id })
    if (!r.data.ok) throw new Error(r.data.error ?? 'Erreur inconnue')
  },

  getSupabaseSchema: async (): Promise<DbTableSchema[]> => {
    const r = await axios.get(`${BASE}/db/supabase/schema`)
    if (!r.data.ok) throw new Error(r.data.error ?? 'Erreur inconnue')
    return r.data.data ?? []
  },

  getDbSecurity: async (): Promise<DbSecurityConfig> => {
    const r = await axios.get(`${BASE}/db/supabase/security`)
    return r.data.data ?? { supabase: {} }
  },

  saveDbSecurity: async (config: DbSecurityConfig): Promise<void> => {
    await axios.post(`${BASE}/db/supabase/security`, config)
  },
}
