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

export interface ConfigData {
  gmail_configured: boolean
  gmail_username: string | null
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

export const api = {
  getStatus: async (): Promise<StatusData> => {
    const r = await axios.get(`${BASE}/status`)
    return r.data.data
  },

  search: async (q: string): Promise<GroupedSearchResponse> => {
    const r = await axios.get(`${BASE}/search`, { params: { q } })
    return r.data.data ?? { groups: [] }
  },

  getRecent: async (source: string, limit = 20, offset = 0): Promise<RecentDoc[]> => {
    const r = await axios.get(`${BASE}/recent`, { params: { source, limit, offset } })
    return r.data.data ?? []
  },

  getConfig: async (): Promise<ConfigData> => {
    const r = await axios.get(`${BASE}/config`)
    return r.data.data
  },

  saveGmail: async (username: string, app_password: string) => {
    await axios.post(`${BASE}/config/gmail`, { username, app_password })
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
}
