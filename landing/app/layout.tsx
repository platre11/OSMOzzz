import type { Metadata } from 'next'
import StyledComponentsRegistry from './registry'
import { LanguageProvider } from '../context/LanguageContext'

export const metadata: Metadata = {
  title: 'OSMOzzz — Local Memory for AI',
  description: 'Give Claude access to all your cloud tools. Gmail, Notion, GitHub, Jira, Linear, Stripe and more — 100% local. Nothing leaves your Mac.',
  openGraph: {
    title: 'OSMOzzz — Local Memory for AI',
    description: 'Give Claude access to all your data. 100% local. Nothing leaves your Mac.',
    type: 'website',
  },
}

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <head>
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <meta name="google-site-verification" content="CuQrOmCvSxySRg7lJtSwEbSDaVYj5mjoxrzADCTyjq8" />
        <link rel="icon" href="/favicon.svg" type="image/svg+xml" />
      </head>
      <body style={{ margin: 0, padding: 0 }}>
        <style>{`
          @media (max-width: 768px) {
            #mobile-block {
              display: flex !important;
            }
            #main-content {
              display: none !important;
            }
          }
        `}</style>
        <div id="mobile-block" style={{
          display: 'none',
          position: 'fixed',
          inset: 0,
          background: '#080a10',
          color: '#e8eaf0',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          gap: '16px',
          textAlign: 'center',
          padding: '32px',
          zIndex: 9999,
          fontFamily: 'system-ui, sans-serif',
        }}>
          <p style={{ fontSize: '32px', margin: 0 }}>🖥️</p>
          <p style={{ fontSize: '18px', fontWeight: 600, margin: 0 }}>OSMOzzz is designed for desktop</p>
          <p style={{ fontSize: '14px', color: '#6b7280', margin: 0 }}>Please open this page on a computer.</p>
        </div>
        <div id="main-content">
          <StyledComponentsRegistry>
            <LanguageProvider>
              {children}
            </LanguageProvider>
          </StyledComponentsRegistry>
        </div>
      </body>
    </html>
  )
}
