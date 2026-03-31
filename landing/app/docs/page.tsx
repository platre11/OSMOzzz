'use client'
import { useState, useEffect, useRef, useCallback } from 'react'
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
  max-width: 860px;
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
      "args": ["mcp"],
      "env": {
        "ORT_DYLIB_PATH": "/opt/homebrew/lib/libonnxruntime.dylib"
      }
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
      "args": ["mcp"],
      "env": {
        "ORT_DYLIB_PATH": "/opt/homebrew/lib/libonnxruntime.dylib"
      }
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
      "args": ["mcp"],
      "env": {
        "ORT_DYLIB_PATH": "/opt/homebrew/lib/libonnxruntime.dylib"
      }
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
        "args": ["mcp"],
        "env": {
          "ORT_DYLIB_PATH": "/opt/homebrew/lib/libonnxruntime.dylib"
        }
      }
    }
  }
}`,
  },
}

const TOOLS = [
  { name: 'search_memory',      cat: 'Recherche',   desc: 'Recherche sémantique vectorielle dans toutes les sources' },
  { name: 'search_emails',      cat: 'Recherche',   desc: 'Recherche dans les emails par mot-clé' },
  { name: 'get_emails_by_date', cat: 'Recherche',   desc: 'Emails filtrés par période ou date' },
  { name: 'read_email',         cat: 'Recherche',   desc: 'Contenu complet d\'un email (par ID)' },
  { name: 'search_messages',    cat: 'Recherche',   desc: 'Recherche dans les iMessages / SMS' },
  { name: 'search_notes',       cat: 'Recherche',   desc: 'Recherche dans Apple Notes' },
  { name: 'search_terminal',    cat: 'Recherche',   desc: 'Recherche dans l\'historique terminal' },
  { name: 'search_calendar',    cat: 'Recherche',   desc: 'Recherche dans Apple Calendrier' },
  { name: 'get_upcoming_events',cat: 'Recherche',   desc: 'Prochains événements du calendrier' },
  { name: 'search_notion',      cat: 'Recherche',   desc: 'Recherche dans les pages Notion' },
  { name: 'search_github',      cat: 'Recherche',   desc: 'Recherche dans les issues & PRs GitHub' },
  { name: 'search_linear',      cat: 'Recherche',   desc: 'Recherche dans les issues Linear' },
  { name: 'search_jira',        cat: 'Recherche',   desc: 'Recherche dans les tickets Jira' },
  { name: 'search_slack',       cat: 'Recherche',   desc: 'Recherche dans les messages Slack' },
  { name: 'search_trello',      cat: 'Recherche',   desc: 'Recherche dans les cartes Trello' },
  { name: 'search_todoist',     cat: 'Recherche',   desc: 'Recherche dans les tâches Todoist' },
  { name: 'search_gitlab',      cat: 'Recherche',   desc: 'Recherche dans les issues GitLab' },
  { name: 'search_airtable',    cat: 'Recherche',   desc: 'Recherche dans les bases Airtable' },
  { name: 'search_obsidian',    cat: 'Recherche',   desc: 'Recherche dans le vault Obsidian' },
  { name: 'find_file',          cat: 'Fichiers',    desc: 'Recherche un fichier par nom ou chemin' },
  { name: 'fetch_content',      cat: 'Fichiers',    desc: 'Lit le contenu d\'un fichier (avec scoring RAG)' },
  { name: 'get_recent_files',   cat: 'Fichiers',    desc: 'Fichiers récemment modifiés' },
  { name: 'list_directory',     cat: 'Fichiers',    desc: 'Liste le contenu d\'un dossier' },
  { name: 'index_files',        cat: 'Fichiers',    desc: 'Déclenche l\'indexation d\'un dossier' },
  { name: 'get_status',         cat: 'Admin',       desc: 'Nombre de documents indexés par source' },
]

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
      { id: 'clients-mcp', label: 'Configurer un client' },
    ],
  },
  {
    label: 'Concepts',
    items: [
      { id: 'mcp',            label: 'C\'est quoi MCP ?' },
      { id: 'privacy',        label: 'Confidentialité' },
      { id: 'tools',          label: 'Les 25 tools' },
    ],
  },
]

// ─── Page ─────────────────────────────────────────────────────────────────────

export default function DocsPage() {
  const [activeId, setActiveId] = useState('installation')
  const [activeClient, setActiveClient] = useState<McpClient>('Claude Desktop')
  const observersRef = useRef<IntersectionObserver[]>([])

  // Scrollspy
  useEffect(() => {
    const allIds = NAV_SECTIONS.flatMap(s => s.items.map(i => i.id))
    observersRef.current.forEach(o => o.disconnect())
    observersRef.current = []

    allIds.forEach(id => {
      const el = document.getElementById(id)
      if (!el) return
      const obs = new IntersectionObserver(
        ([entry]) => { if (entry.isIntersecting) setActiveId(id) },
        { rootMargin: '-30% 0px -60% 0px' }
      )
      obs.observe(el)
      observersRef.current.push(obs)
    })
    return () => observersRef.current.forEach(o => o.disconnect())
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
                L'installeur place le binaire dans <InlineCode>/usr/local/bin/osmozzz</InlineCode> et
                copie la librairie ONNX Runtime nécessaire.
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
                OSMOzzz est conçu pour que vous gardiez le contrôle total sur ce que voit votre client IA.
                Quatre mécanismes indépendants :
              </DocP>
              <DocH3>Filtre de confidentialité</DocH3>
              <DocP>
                Masque automatiquement les patterns sensibles dans les résultats :
                numéros de carte bancaire, IBAN, clés API, adresses email, numéros de téléphone.
                Configurable depuis le dashboard → Actions MCP → Confidentialité.
              </DocP>
              <DocH3>Alias d'identité</DocH3>
              <DocP>
                Si configuré, OSMOzzz remplace les vrais noms par des alias avant envoi au client IA.
                "Jean Dupont" devient "Collaborateur-A". Sans configuration, les données
                passent sans transformation.
              </DocP>
              <DocH3>Liste noire</DocH3>
              <DocP>
                Excluez des documents, des expéditeurs, des domaines ou des chemins de fichiers
                entiers des résultats. Ces données ne sont jamais transmises, même si elles
                correspondent à la requête.
              </DocP>
              <DocH3>Contrôle d'accès par source</DocH3>
              <DocP>
                Activez ou désactivez l'accès à chaque source individuellement.
                Par défaut, les sources sensibles (iMessage, Terminal, historique de navigation)
                peuvent être désactivées si vous ne souhaitez pas que votre client IA y ait accès.
              </DocP>
            </DocSection>

            <DocSection id="tools">
              <DocH2>Les 25 tools MCP</DocH2>
              <DocP>
                OSMOzzz expose 25 tools à votre client IA. Il les découvre automatiquement
                au démarrage — vous n'avez rien à configurer.
              </DocP>
              <ToolTable>
                <thead>
                  <tr>
                    <Th>Tool</Th>
                    <Th>Catégorie</Th>
                    <Th>Description</Th>
                  </tr>
                </thead>
                <tbody>
                  {TOOLS.map(t => (
                    <tr key={t.name}>
                      <TdCode>{t.name}</TdCode>
                      <Td>
                        <Badge $color={t.cat === 'Recherche' ? 'purple' : t.cat === 'Fichiers' ? 'green' : undefined}>
                          {t.cat}
                        </Badge>
                      </Td>
                      <Td>{t.desc}</Td>
                    </tr>
                  ))}
                </tbody>
              </ToolTable>
            </DocSection>

          </Content>
        </Body>
      </Shell>
    </>
  )
}
