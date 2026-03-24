import type { Metadata } from 'next'
import StyledComponentsRegistry from './registry'
import { LanguageProvider } from '../context/LanguageContext'

export const metadata: Metadata = {
  title: 'OSMOzzz — Local Memory for AI',
  description: 'Give Claude access to all your data. Emails, files, messages, calendar, code — 100% local. Nothing leaves your Mac.',
  openGraph: {
    title: 'OSMOzzz — Local Memory for AI',
    description: 'Give Claude access to all your data. 100% local. Nothing leaves your Mac.',
    type: 'website',
  },
}

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body style={{ margin: 0, padding: 0 }}>
        <StyledComponentsRegistry>
          <LanguageProvider>
            {children}
          </LanguageProvider>
        </StyledComponentsRegistry>
      </body>
    </html>
  )
}
