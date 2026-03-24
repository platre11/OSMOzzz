'use client'
import { useEffect, useRef } from 'react'
import styled, { createGlobalStyle } from 'styled-components'
import { useLang } from '../context/LanguageContext'
import HeroBlock from './HeroBlock'

// ── Global ────────────────────────────────────────────────────────────────────

const GlobalStyle = createGlobalStyle`
  *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
  :root {
    --bg: #0a0b0f;
    --bg2: #0f1117;
    --border: #1f2230;
    --text: #e8eaf0;
    --muted: #6b7280;
    --accent: #5b5ef4;
    --accent-glow: rgba(91, 94, 244, 0.15);
  }
  html { scroll-behavior: smooth; }
  body {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
    background: var(--bg);
    color: var(--text);
    line-height: 1.6;
    -webkit-font-smoothing: antialiased;
  }
`

// ── Nav ───────────────────────────────────────────────────────────────────────

const NavBar = styled.nav`
  position: sticky; top: 0; z-index: 100;
`
const NavInner = styled.div`
  max-width: 1000px; margin: 0 auto;
  padding: 0 24px; height: 56px;
  display: flex; align-items: center; justify-content: space-between;
`
const Logo = styled.span`
  font-size: 17px; font-weight: 800; color: var(--accent);
  letter-spacing: -0.03em;
`
const NavRight = styled.div`
  display: flex; gap: 12px; align-items: center;
`
const LangSwitcher = styled.div`
  display: flex; gap: 2px;
  background: rgba(255,255,255,.08);
  border-radius: 8px; padding: 3px;
`
const LangBtn = styled.button<{ $active: boolean }>`
  font-size: 12px; font-weight: 600;
  padding: 4px 10px; border-radius: 6px;
  border: none; cursor: pointer;
  background: ${p => p.$active ? '#fff' : 'transparent'};
  color: ${p => p.$active ? '#1a1d23' : '#9ca3af'};
  transition: background .15s, color .15s;
`
const BtnGhost = styled.a`
  display: flex; align-items: center; gap: 6px;
  padding: 6px 14px; border-radius: 8px;
  border: 1px solid var(--border);
  background: transparent; color: var(--muted);
  text-decoration: none; font-size: 13px; font-weight: 500;
  transition: all .15s;
  &:hover { border-color: #374151; color: var(--text); }
`

// ── Hero ──────────────────────────────────────────────────────────────────────

const HeroWrapper = styled.div`
  position: relative;
  overflow: hidden;
`
const HeroBgSvg = styled.svg`
  position: absolute;
  top: 0; left: 0;
  width: 100%; height: 100%;
  pointer-events: none;
  z-index: 0;
`
const HeroSection = styled.section`
  position: relative;
  z-index: 1;
  max-width: 1000px; margin: 0 auto;
  padding: 96px 24px 80px;
  text-align: center;
  @media (max-width: 640px) { padding: 64px 20px 56px; }
`
const HeroBadge = styled.div`
  display: inline-block; margin-bottom: 24px;
  padding: 4px 14px; border-radius: 99px;
  border: 1px solid var(--border); background: var(--bg2);
  font-size: 12px; color: var(--muted); font-weight: 500;
`
const H1 = styled.h1`
  font-size: clamp(42px, 7vw, 52px);
  font-weight: 800; line-height: 1.1;
  letter-spacing: -0.04em; color: #fff;
  margin-bottom: 24px;
`
const HeroSub = styled.p`
  font-size: clamp(13px, 2.5vw, 10px);
  color: var(--muted); max-width: 600px;
  margin: 0 auto 20px; line-height: 1.7;
`
const HeroActions = styled.div`
  display: flex; flex-direction: column; align-items: center; gap: 10px; margin-bottom: 32px;
`
const BtnDownload = styled.a`
  display: inline-flex; align-items: center; gap: 10px;
  padding: 16px 32px; border-radius: 12px;
  background: var(--accent); color: #fff; text-decoration: none;
  font-size: 16px; font-weight: 700; transition: all .15s;
  box-shadow: 0 0 0 1px rgba(91,94,244,.5), 0 8px 32px rgba(91,94,244,.3);
  &:hover { background: #4a4de3; transform: translateY(-2px); box-shadow: 0 0 0 1px rgba(91,94,244,.5), 0 12px 40px rgba(91,94,244,.4); }
`
const HeroMeta = styled.span`
  font-size: 13px; color: var(--muted);
`
const ShieldCanvas = styled.div`
  width: 320px; height: 320px;
  margin: 0 auto;
  flex-shrink: 0;
  canvas { display: block; }
  @media (max-width: 640px) { width: 220px; height: 220px; }
`

// ── Sections ──────────────────────────────────────────────────────────────────

const Section = styled.section<{ $dark?: boolean; $cta?: boolean }>`
  padding: 80px 24px;
  ${p => p.$dark && `background: var(--bg2); border-top: 1px solid var(--border); border-bottom: 1px solid var(--border);`}
  ${p => p.$cta && `background: linear-gradient(180deg, var(--bg) 0%, #0d0e1a 100%);`}
`
const Container = styled.div`
  max-width: 1000px; margin: 0 auto;
`
const H2 = styled.h2`
  font-size: clamp(28px, 4vw, 40px); font-weight: 800;
  letter-spacing: -0.03em; color: #fff; margin-bottom: 12px;
`
const SectionSub = styled.p`
  font-size: 16px; color: var(--muted); margin-bottom: 48px;
`

// ── Vision ────────────────────────────────────────────────────────────────────

const VisionIntro = styled.div`
  margin-bottom: 48px; max-width: 680px;
`
const VisionLead = styled.p`
  font-size: 16px; color: var(--muted); line-height: 1.8;
`
const VisionGrid = styled.div`
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 16px;
`
const VisionFeat = styled.div`
  display: flex; gap: 16px; align-items: flex-start;
  background: var(--bg); border: 1px solid var(--border);
  border-radius: 12px; padding: 20px;
`
const VisionFeatTitle = styled.h3`
  font-size: 14px; font-weight: 700; color: #fff; margin-bottom: 6px;
`
const VisionFeatDesc = styled.p`
  font-size: 13px; color: var(--muted); line-height: 1.6; margin: 0;
`

// ── Comparison ────────────────────────────────────────────────────────────────

const CompareGrid = styled.div`
  display: grid; grid-template-columns: 1fr 1fr; gap: 20px;
  @media (max-width: 640px) { grid-template-columns: 1fr; }
`
const CompareCol = styled.div<{ $bad?: boolean }>`
  border-radius: 16px; padding: 32px;
  border: 1px solid ${p => p.$bad ? 'rgba(239,68,68,.2)' : 'rgba(22,163,74,.2)'};
  background: ${p => p.$bad ? 'rgba(239,68,68,.04)' : 'rgba(22,163,74,.04)'};
`
const CompareBadge = styled.span<{ $bad?: boolean }>`
  display: inline-block;
  font-size: 12px; font-weight: 700;
  padding: 5px 14px; border-radius: 99px;
  letter-spacing: 0.02em;
  background: ${p => p.$bad ? 'rgba(239,68,68,.12)' : 'rgba(22,163,74,.12)'};
  color: ${p => p.$bad ? '#f87171' : '#4ade80'};
`
const CompareList = styled.ul`
  list-style: none; display: flex; flex-direction: column; gap: 14px; margin-top: 24px;
`
const CompareItem = styled.li<{ $bad?: boolean }>`
  display: flex; align-items: flex-start; gap: 12px;
  font-size: 14px; line-height: 1.55; color: var(--muted);
`
const CompareIcon = styled.span<{ $bad?: boolean }>`
  font-size: 13px; font-weight: 800; flex-shrink: 0; margin-top: 1px;
  color: ${p => p.$bad ? '#f87171' : '#4ade80'};
`
const CompareConclusion = styled.div`
  margin-top: 40px; text-align: center; padding: 28px 32px;
  border-radius: 14px;
  background: linear-gradient(135deg, rgba(91,94,244,.08), rgba(22,163,74,.06));
  border: 1px solid rgba(91,94,244,.2);
  p { font-size: 20px; font-weight: 700; color: #fff; letter-spacing: -0.02em; }
`

// ── Sources ───────────────────────────────────────────────────────────────────

const SourcesSplit = styled.div`
  display: grid; grid-template-columns: 1fr auto 1fr; gap: 40px; align-items: start;
  @media (max-width: 640px) { grid-template-columns: 1fr; }
`
const SourcesDivider = styled.div`
  width: 1px; background: var(--border); align-self: stretch; margin-top: 40px;
  @media (max-width: 640px) { display: none; }
`
const SourcesColHeader = styled.div<{ $type: 'local' | 'cloud' }>`
  display: flex; align-items: center; gap: 8px;
  font-size: 11px; font-weight: 700; text-transform: uppercase;
  letter-spacing: 0.08em; margin-bottom: 16px;
  color: ${p => p.$type === 'local' ? '#4ade80' : '#a5b4fc'};
`
const SourcesDot = styled.span<{ $type: 'local' | 'cloud' }>`
  width: 7px; height: 7px; border-radius: 50%; flex-shrink: 0;
  background: ${p => p.$type === 'local' ? '#4ade80' : '#a5b4fc'};
  box-shadow: ${p => p.$type === 'local' ? '0 0 6px #4ade80' : '0 0 6px #a5b4fc'};
`
const SourcesCards = styled.div`
  display: grid; grid-template-columns: repeat(auto-fill, minmax(120px, 1fr)); gap: 8px;
`
const SourceCard = styled.div<{ $type: 'local' | 'cloud' }>`
  display: flex; align-items: center; gap: 8px;
  padding: 10px 14px; border-radius: 10px; font-size: 13px; font-weight: 500;
  transition: transform .15s, border-color .15s; cursor: default;
  span { color: var(--text); }
  background: ${p => p.$type === 'local' ? 'rgba(22,163,74,.07)' : 'rgba(91,94,244,.07)'};
  border: 1px solid ${p => p.$type === 'local' ? 'rgba(22,163,74,.2)' : 'rgba(91,94,244,.2)'};
  &:hover {
    transform: translateY(-2px);
    border-color: ${p => p.$type === 'local' ? 'rgba(22,163,74,.4)' : 'rgba(91,94,244,.4)'};
  }
`

// ── CTA ───────────────────────────────────────────────────────────────────────

const CtaCenter = styled.div`text-align: center;`

// ── Footer ────────────────────────────────────────────────────────────────────

const FooterEl = styled.footer`
  border-top: 1px solid var(--border); padding: 24px;
`
const FooterInner = styled.div`
  max-width: 1000px; margin: 0 auto;
  display: flex; align-items: center; justify-content: space-between;
  flex-wrap: wrap; gap: 12px;
  @media (max-width: 640px) { flex-direction: column; align-items: flex-start; }
`
const FooterLink = styled.a`
  color: #6b7280; font-size: 13px; text-decoration: none;
  transition: color .15s;
  &:hover { color: var(--text); }
`

// ── GitHub SVG ────────────────────────────────────────────────────────────────

const GithubIcon = () => (
  <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
    <path d="M12 2C6.477 2 2 6.477 2 12c0 4.42 2.865 8.166 6.839 9.489.5.092.682-.217.682-.482 0-.237-.008-.866-.013-1.7-2.782.603-3.369-1.342-3.369-1.342-.454-1.155-1.11-1.463-1.11-1.463-.908-.62.069-.608.069-.608 1.003.07 1.531 1.03 1.531 1.03.892 1.529 2.341 1.087 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.11-4.555-4.943 0-1.091.39-1.984 1.029-2.683-.103-.253-.446-1.27.098-2.647 0 0 .84-.269 2.75 1.025A9.578 9.578 0 0112 6.836c.85.004 1.705.114 2.504.336 1.909-1.294 2.747-1.025 2.747-1.025.546 1.377.202 2.394.1 2.647.64.699 1.028 1.592 1.028 2.683 0 3.842-2.339 4.687-4.566 4.935.359.309.678.919.678 1.852 0 1.336-.012 2.415-.012 2.741 0 .267.18.578.688.48C19.138 20.163 22 16.418 22 12c0-5.523-4.477-10-10-10z" />
  </svg>
)

const DownloadIcon = () => (
  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
    <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4" />
    <polyline points="7 10 12 15 17 10" />
    <line x1="12" y1="15" x2="12" y2="3" />
  </svg>
)

// ── Page ──────────────────────────────────────────────────────────────────────

export default function HomePage() {
  const { lang, setLang, t } = useLang()
  const shieldRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!shieldRef.current) return
    let cleanup: (() => void) | undefined
    import('./shield3d').then(({ initShield }) => {
      if (shieldRef.current) cleanup = initShield(shieldRef.current)
    })
    return () => cleanup?.()
  }, [])

  const heroLines = t('heroTitle').split('\n')

  return (
    <>
      <GlobalStyle />

      {/* HERO + NAV */}
      <HeroWrapper>
        <HeroBgSvg viewBox="0 0 1440 800" preserveAspectRatio="none" xmlns="http://www.w3.org/2000/svg">
          <defs>
            <linearGradient id="heroGrad" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor="#5b5ef4" stopOpacity="0.2" />
              <stop offset="100%" stopColor="#5b5ef4" stopOpacity="0.02" />
            </linearGradient>
          </defs>
          <path
            d="M 0,0 H 1440 V 280 C 1080,280 900,520 720,520 C 540,520 360,280 0,280 Z"
            fill="url(#heroGrad)"
          />
        </HeroBgSvg>
        <NavBar>
          <NavInner>
            <Logo>OSMOzzz</Logo>
            <NavRight>
              <LangSwitcher>
                <LangBtn $active={lang === 'en'} onClick={() => setLang('en')}>EN</LangBtn>
                <LangBtn $active={lang === 'fr'} onClick={() => setLang('fr')}>FR</LangBtn>
              </LangSwitcher>
              <BtnGhost href="https://github.com/platre11/OSMOzzz" target="_blank" rel="noreferrer">
                <GithubIcon />
                {t('navGithub')}
              </BtnGhost>
            </NavRight>
          </NavInner>
        </NavBar>
        <HeroSection>
          <HeroBadge>{t('heroBadge')}</HeroBadge>
          <H1>
            {heroLines[0]}
            {heroLines[1] && <><br />{heroLines[1]}</>}
          </H1>
          <HeroBlock />
          <HeroSub>{t('heroSub')}</HeroSub>
          <HeroActions>
            <BtnDownload href="https://github.com/platre11/OSMOzzz/releases/latest/download/osmozzz.pkg">
              <DownloadIcon />
              {t('heroDownload')}
            </BtnDownload>
            <HeroMeta>{t('heroMeta')}</HeroMeta>
          </HeroActions>
        </HeroSection>
      </HeroWrapper>

      {/* VISION */}
      <Section $dark>
        <Container>
          <VisionIntro>
            <H2>{t('visionTitle')}</H2>
            <VisionLead>{t('visionLead')}</VisionLead>
          </VisionIntro>
          <VisionGrid>
            {([
              [t('visionF1Title'), t('visionF1Desc')],
              [t('visionF2Title'), t('visionF2Desc')],
              [t('visionF3Title'), t('visionF3Desc')],
              [t('visionF4Title'), t('visionF4Desc')],
              [t('visionF5Title'), t('visionF5Desc')],
            ] as [string, string][]).map(([title, desc]) => (
              <VisionFeat key={title}>
                <div>
                  <VisionFeatTitle>{title}</VisionFeatTitle>
                  <VisionFeatDesc>{desc}</VisionFeatDesc>
                </div>
              </VisionFeat>
            ))}
          </VisionGrid>
        </Container>
      </Section>

      {/* COMPARISON */}
      <Section>
        <Container>
          <H2>{t('compareTitle')}</H2>
          <SectionSub>{t('compareSub')}</SectionSub>
          <CompareGrid>
            <CompareCol $bad>
              <CompareBadge $bad>{t('compareWithoutBadge')}</CompareBadge>
              <CompareList>
                {([
                  t('compareWithout1'),
                  t('compareWithout2'),
                  t('compareWithout3'),
                  t('compareWithout4'),
                  t('compareWithout5'),
                ]).map((item, i) => (
                  <CompareItem key={i} $bad>
                    <CompareIcon $bad>✗</CompareIcon>
                    <span>{item}</span>
                  </CompareItem>
                ))}
              </CompareList>
            </CompareCol>
            <CompareCol>
              <CompareBadge>{t('compareWithBadge')}</CompareBadge>
              <CompareList>
                {([
                  t('compareWith1'),
                  t('compareWith2'),
                  t('compareWith3'),
                  t('compareWith4'),
                  t('compareWith5'),
                ]).map((item, i) => (
                  <CompareItem key={i}>
                    <CompareIcon>✓</CompareIcon>
                    <span>{item}</span>
                  </CompareItem>
                ))}
              </CompareList>
            </CompareCol>
          </CompareGrid>
          <CompareConclusion>
            <p>{t('compareConclusion')}</p>
          </CompareConclusion>
        </Container>
      </Section>

      {/* SOURCES */}
      <Section $dark>
        <Container>
          <H2>{t('sourcesTitle')}</H2>
          <SectionSub>{t('sourcesSub')}</SectionSub>
          <SourcesSplit>
            <div>
              <SourcesColHeader $type="local">
                <SourcesDot $type="local" />
                {t('sourcesLocalLabel')}
              </SourcesColHeader>
              <SourcesCards>
                {['Gmail','Chrome','Safari','iMessage','Apple Notes','Calendar','Terminal','Files'].map(name => (
                  <SourceCard key={name} $type="local"><span>{name}</span></SourceCard>
                ))}
              </SourcesCards>
            </div>
            <SourcesDivider />
            <div>
              <SourcesColHeader $type="cloud">
                <SourcesDot $type="cloud" />
                {t('sourcesCloudLabel')}
              </SourcesColHeader>
              <SourcesCards>
                {['Notion','GitHub','Linear','Jira','Supabase'].map(name => (
                  <SourceCard key={name} $type="cloud"><span>{name}</span></SourceCard>
                ))}
              </SourcesCards>
            </div>
          </SourcesSplit>
        </Container>
      </Section>

      {/* CTA */}
      <Section $cta>
        <Container>
          <CtaCenter>
            <H2>{t('ctaTitle')}</H2>
            <SectionSub>{t('ctaSub')}</SectionSub>
            <BtnDownload
              href="https://github.com/platre11/OSMOzzz/releases/latest/download/osmozzz.pkg"
              style={{ margin: '32px auto 0', display: 'inline-flex' }}
            >
              <DownloadIcon />
              {t('ctaDownload')}
            </BtnDownload>
            <br />
            <BtnGhost
              href="https://github.com/platre11/OSMOzzz"
              target="_blank"
              rel="noreferrer"
              style={{ marginTop: '12px', display: 'inline-flex' }}
            >
              {t('ctaGithub')}
            </BtnGhost>
          </CtaCenter>
        </Container>
      </Section>

      {/* FOOTER */}
      <FooterEl>
        <FooterInner>
          <Logo style={{ fontSize: '14px' }}>OSMOzzz</Logo>
          <span style={{ color: '#4b5563', fontSize: '13px' }}>{t('footerLicense')}</span>
          <FooterLink href="https://github.com/platre11/osmozzz" target="_blank" rel="noreferrer">
            GitHub →
          </FooterLink>
        </FooterInner>
      </FooterEl>
    </>
  )
}
