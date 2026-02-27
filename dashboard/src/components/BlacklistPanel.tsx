import styled from 'styled-components'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '../api'
import type { BlacklistEntry } from '../api'

const SOURCE_LABELS: Record<string, string> = {
  email: 'Gmail', chrome: 'Chrome', file: 'Fichiers', imessage: 'iMessage',
  safari: 'Safari', notes: 'Notes', terminal: 'Terminal', calendar: 'Calendrier',
  any: 'Toutes sources',
}

const Overlay = styled.div`
  position: fixed;
  inset: 0;
  background: rgba(0,0,0,.35);
  z-index: 100;
  display: flex;
  align-items: flex-start;
  justify-content: flex-end;
  padding: 16px;
`

const Panel = styled.div`
  background: #fff;
  border-radius: 16px;
  box-shadow: 0 8px 32px rgba(0,0,0,.18);
  width: 420px;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
`

const Header = styled.div`
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 18px 20px 14px;
  border-bottom: 1px solid #f3f4f6;
`

const Title = styled.p`
  font-size: 14px;
  font-weight: 700;
  color: #1a1d23;
`

const CloseBtn = styled.button`
  background: none;
  border: none;
  font-size: 18px;
  cursor: pointer;
  color: #9ca3af;
  line-height: 1;
  &:hover { color: #374151; }
`

const Empty = styled.p`
  padding: 32px;
  text-align: center;
  color: #9ca3af;
  font-size: 13px;
`

const Scroll = styled.div`
  overflow-y: auto;
  flex: 1;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
`

const Item = styled.div`
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: #f9fafb;
  border: 1px solid #f3f4f6;
  border-radius: 10px;
  padding: 10px 14px;
  gap: 10px;
`

const ItemText = styled.div`
  flex: 1;
  min-width: 0;
`

const ItemKind = styled.span`
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
  color: #9ca3af;
  letter-spacing: .04em;
`

const ItemId = styled.p`
  font-size: 12px;
  color: #374151;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  margin-top: 2px;
`

const ItemContent = styled.p`
  font-size: 11px;
  color: #9ca3af;
  margin-top: 3px;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
`

const UnbanBtn = styled.button`
  padding: 4px 10px;
  border-radius: 6px;
  font-size: 11px;
  font-weight: 600;
  border: 1px solid #10b981;
  background: #f0fdf4;
  color: #059669;
  cursor: pointer;
  white-space: nowrap;
  transition: all .15s;
  &:hover { background: #dcfce7; }
`

export const BannisBtn = styled.button`
  padding: 5px 12px;
  border-radius: 8px;
  font-size: 12px;
  font-weight: 500;
  border: 1px solid #e5e7eb;
  background: #fff;
  color: #6b7280;
  cursor: pointer;
  transition: all .15s;
  &:hover { background: #f3f4f6; }
`

interface Props {
  source: string   // source to filter, or 'all' for global
  onClose: () => void
}

export default function BlacklistPanel({ source, onClose }: Props) {
  const queryClient = useQueryClient()
  const { data, isLoading, refetch } = useQuery({
    queryKey: ['blacklist'],
    queryFn: api.getBlacklist,
  })

  // For URL bans: source comes from the vault lookup (real source of the doc)
  // For source bans: source is stored directly
  // Filter strictly by source — no leakage between tabs
  const filtered = (data?.entries ?? []).filter(e =>
    source === 'all' ? true : e.source === source
  )

  async function doUnban(entry: BlacklistEntry) {
    if (entry.kind === 'url') await api.unbanUrl(entry.identifier)
    else await api.unbanSourceItem(entry.source, entry.identifier)
    await refetch()
    queryClient.invalidateQueries({ queryKey: ['recent'] })
    queryClient.invalidateQueries({ queryKey: ['search'] })
  }

  const label = source === 'all' ? 'tous' : (SOURCE_LABELS[source] ?? source)

  return (
    <Overlay onClick={onClose}>
      <Panel onClick={e => e.stopPropagation()}>
        <Header>
          <Title>Éléments bannis — {label}</Title>
          <CloseBtn onClick={onClose}>✕</CloseBtn>
        </Header>

        {isLoading
          ? <Empty>Chargement...</Empty>
          : filtered.length === 0
            ? <Empty>Aucun élément banni{source !== 'all' ? ' dans cette source' : ''}.</Empty>
            : <Scroll>
                {filtered.map((entry, i) => (
                  <Item key={i}>
                    <ItemText>
                      <ItemKind>
                        {entry.kind === 'url'
                          ? `Document banni (${SOURCE_LABELS[entry.source] ?? entry.source})`
                          : `Tout de (${SOURCE_LABELS[entry.source] ?? entry.source})`
                        }
                      </ItemKind>
                      {entry.title
                        ? <ItemId title={entry.identifier}>{entry.title}</ItemId>
                        : <ItemId title={entry.identifier}>{entry.identifier}</ItemId>
                      }
                      {entry.content && (
                        <ItemContent>{entry.content}</ItemContent>
                      )}
                    </ItemText>
                    <UnbanBtn onClick={() => doUnban(entry)}>Débannir</UnbanBtn>
                  </Item>
                ))}
              </Scroll>
        }
      </Panel>
    </Overlay>
  )
}
