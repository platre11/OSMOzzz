import React from 'react'
import styled from 'styled-components'

export const Highlight = styled.mark`
  background: rgba(91, 94, 244, 0.18);
  color: inherit;
  border-radius: 3px;
  padding: 0 1px;
`

export const InlineLink = styled.a`
  color: #5b5ef4;
  text-decoration: underline;
  text-underline-offset: 2px;
  word-break: break-all;
  &:hover { color: #4a4de3; }
`

const LINK_RE = /(https?:\/\/[^\s<>"'[\]()]+|[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,})/g

export function highlightText(text: string, query: string): React.ReactNode {
  if (!query || query.length < 2) return text
  const idx = text.toLowerCase().indexOf(query.toLowerCase())
  if (idx === -1) return text
  return (
    <>
      {text.slice(0, idx)}
      <Highlight>{text.slice(idx, idx + query.length)}</Highlight>
      {highlightText(text.slice(idx + query.length), query)}
    </>
  )
}

export function renderEmailContent(text: string, query: string): React.ReactNode {
  const parts = text.split(LINK_RE)
  return (
    <>
      {parts.map((part, i) => {
        if (/^https?:\/\//.test(part)) {
          return (
            <InlineLink key={i} href={part} target="_blank" rel="noreferrer" onClick={e => e.stopPropagation()}>
              {highlightText(part, query)}
            </InlineLink>
          )
        }
        if (/^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$/.test(part)) {
          return (
            <InlineLink key={i} href={`mailto:${part}`} onClick={e => e.stopPropagation()}>
              {highlightText(part, query)}
            </InlineLink>
          )
        }
        return <span key={i}>{highlightText(part, query)}</span>
      })}
    </>
  )
}
