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

export interface QueryHistoryEntry {
  ts: number
  peer_id: string
  peer_name: string
  query: string
  results_count: number
  blocked: boolean
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
  credit_card: boolean
  iban: boolean
  api_keys: boolean
  email: boolean
  phone: boolean
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
    return r.data.data ?? { credit_card: true, iban: true, api_keys: true, email: false, phone: false }
  },

  setPrivacy: async (config: PrivacyConfig): Promise<void> => {
    await axios.post(`${BASE}/privacy`, config)
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

  getPermissions: async (): Promise<{ jira: boolean; github: boolean; linear: boolean; notion: boolean; email: boolean }> => {
    const r = await axios.get(`${BASE}/permissions`)
    return r.data.data ?? { jira: false, github: false, linear: false, notion: false, email: false }
  },

  savePermissions: async (perms: { jira: boolean; github: boolean; linear: boolean; notion: boolean; email: boolean }): Promise<void> => {
    await axios.post(`${BASE}/permissions`, perms)
  },

  // ─── Accès sources MCP ───────────────────────────────────────────────────────

  getSourceAccess: async (): Promise<{
    email: boolean; imessage: boolean; chrome: boolean; safari: boolean;
    notes: boolean; calendar: boolean; terminal: boolean; file: boolean;
    notion: boolean; github: boolean; linear: boolean; jira: boolean;
  }> => {
    const r = await axios.get(`${BASE}/source-access`)
    return r.data.data ?? {
      email: true, imessage: true, chrome: true, safari: true,
      notes: true, calendar: true, terminal: true, file: true,
      notion: true, github: true, linear: true, jira: true,
    }
  },

  getAudit: async (limit = 200): Promise<Array<{
    ts: number; tool: string; query: string; results: number; blocked: boolean; data?: string;
  }>> => {
    const r = await axios.get(`${BASE}/audit`, { params: { limit } })
    return r.data.data ?? []
  },

  // ─── Alias Engine ────────────────────────────────────────────────────────────

  getAliases: async (): Promise<Array<{ real: string; alias: string }>> => {
    const r = await axios.get(`${BASE}/aliases`)
    return r.data.data ?? []
  },

  saveAliases: async (aliases: Array<{ real: string; alias: string }>): Promise<void> => {
    await axios.post(`${BASE}/aliases`, { aliases })
  },

  saveSourceAccess: async (access: {
    email: boolean; imessage: boolean; chrome: boolean; safari: boolean;
    notes: boolean; calendar: boolean; terminal: boolean; file: boolean;
    notion: boolean; github: boolean; linear: boolean; jira: boolean;
  }): Promise<void> => {
    await axios.post(`${BASE}/source-access`, access)
  },
}
