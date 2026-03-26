'use client'
import { useEffect, useRef, useState } from 'react'
import styled, { createGlobalStyle, keyframes, css } from 'styled-components'
import { useLang } from '../context/LanguageContext'
import HeroBlock from './HeroBlock'
import ShieldRadar from './ShieldRadar'
import CompareSnake from './CompareSnake'

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
  font-size: 17px; font-weight: 800;
  letter-spacing: -0.03em;
  color: #fff;

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
const DownloadRow = styled.div`
  display: flex; gap: 12px; justify-content: center; flex-wrap: wrap;
`
const BtnDownload = styled.a`
  display: inline-flex; align-items: center; gap: 10px;
  padding: 14px 28px; border-radius: 12px;
  background: var(--accent); color: #fff; text-decoration: none;
  font-size: 15px; font-weight: 700; transition: all .15s;
  box-shadow: 0 0 0 1px rgba(91,94,244,.5), 0 8px 32px rgba(91,94,244,.3);
  &:hover { background: #4a4de3; transform: translateY(-2px); box-shadow: 0 0 0 1px rgba(91,94,244,.5), 0 12px 40px rgba(91,94,244,.4); }
`
const BtnDownloadWin = styled(BtnDownload)`
  background: transparent;
  border: 1.5px solid rgba(255,255,255,.18);
  box-shadow: none;
  color: rgba(255,255,255,.85);
  &:hover { background: rgba(255,255,255,.07); box-shadow: none; }
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

const VisionLayout = styled.div`
  display: flex;
  align-items: center;
  gap: 72px;
  @media (max-width: 760px) { flex-direction: column; gap: 40px; }
`
const VisionIntro = styled.div`
  flex: 1;
  min-width: 0;
`
const VisionLead = styled.p`
  font-size: 16px; color: var(--muted); line-height: 1.9;
  margin-top: 24px;
`

// ── Comparison keyframes ──────────────────────────────────────────────────────

const dotPulse = keyframes`
  0%, 100% { box-shadow: 0 0 6px rgba(91,94,244,.7), 0 0 0 0 rgba(91,94,244,.4); }
  50%       { box-shadow: 0 0 10px rgba(91,94,244,1), 0 0 0 6px rgba(91,94,244,0); }
`
const rowFadeIn = keyframes`
  from { opacity: 0; transform: translateX(8px); }
  to   { opacity: 1; transform: translateX(0); }
`

// ── Comparison ────────────────────────────────────────────────────────────────

const CompareTable = styled.div`
  border-radius: 16px; overflow: hidden;
  border: 1px solid rgba(255,255,255,.07);
`
const CompareHeaderRow = styled.div`
  display: grid; grid-template-columns: 1fr 1fr;
`
const CompareHeaderBad = styled.div`
  padding: 20px 28px;
  border-right: 1px solid rgba(255,255,255,.07);
  background: rgba(255,255,255,.02);
  display: flex; align-items: center; gap: 10px;
`
const CompareHeaderGood = styled.div`
  padding: 20px 28px;
  background: rgba(91,94,244,.07);
  display: flex; align-items: center; gap: 10px;
`
const CompareHeaderLabel = styled.span<{ $good?: boolean }>`
  font-size: 11px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase;
  color: ${p => p.$good ? '#a5b4fc' : 'rgba(107,114,128,.7)'};
`
const CompareHeaderDot = styled.span<{ $good?: boolean }>`
  width: 6px; height: 6px; border-radius: 50%; flex-shrink: 0;
  background: ${p => p.$good ? '#5b5ef4' : 'rgba(107,114,128,.4)'};
  ${p => p.$good && css`animation: ${dotPulse} 2s ease-in-out infinite;`}
`
const CompareBody = styled.div`
  display: grid; grid-template-columns: 1fr 1fr;
`
const CompareBadCol = styled.div`
  border-right: 1px solid rgba(255,255,255,.05);
`
const CompareGoodCol = styled.div``
const CompareRow = styled.div<{ $index?: number }>`
  border-top: 1px solid rgba(255,255,255,.05);
  animation: ${rowFadeIn} .4s ease both;
  animation-delay: ${p => `${(p.$index ?? 0) * 80}ms`};
  &:hover { background-color: rgba(255,255,255,.015); }
`
const CompareCellBad = styled.div`
  padding: 20px 28px;
  display: flex; align-items: flex-start; gap: 12px;
  font-size: 13.5px; line-height: 1.6; color: rgba(107,114,128,.7);
`
const CompareCellGood = styled.div`
  padding: 20px 28px;
  display: flex; align-items: flex-start; gap: 12px;
  font-size: 13.5px; line-height: 1.6; color: #c8cdd8;
`
const CellIcon = styled.span<{ $good?: boolean }>`
  flex-shrink: 0; margin-top: 2px;
  font-size: 11px; font-weight: 900;
  color: ${p => p.$good ? '#4ade80' : 'rgba(239,68,68,.5)'};
`
const CompareConclusion = styled.div`
  margin-top: 16px; padding: 32px 40px;
  border-radius: 16px; position: relative; overflow: hidden;
  background: #0d0e18;
  border: 1px solid rgba(91,94,244,.2);
  display: flex; align-items: center; justify-content: center;
  &::before {
    content: ''; position: absolute;
    top: 0; left: 50%; transform: translateX(-50%);
    width: 500px; height: 1px;
    background: linear-gradient(90deg, transparent, rgba(91,94,244,.6), transparent);
  }
  &::after {
    content: ''; position: absolute;
    top: -80px; left: 50%; transform: translateX(-50%);
    width: 400px; height: 160px;
    background: radial-gradient(ellipse, rgba(91,94,244,.12) 0%, transparent 70%);
  }
  p { font-size: 19px; font-weight: 700; color: #fff; letter-spacing: -0.02em; position: relative; z-index: 1; }
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
  display: flex; flex-wrap: wrap; gap: 10px;
`
const SourceCard = styled.div<{ $type: 'local' | 'cloud' }>`
  padding: 8px 18px; border-radius: 999px;
  font-size: 13px; font-weight: 600; letter-spacing: .02em;
  font-family: 'SF Mono', 'Fira Code', monospace;
  transition: transform .15s, box-shadow .15s; cursor: default;
  color: ${p => p.$type === 'local' ? 'rgba(74,222,128,.9)' : 'rgba(139,148,255,.9)'};
  background: ${p => p.$type === 'local' ? 'rgba(22,163,74,.1)' : 'rgba(91,94,244,.1)'};
  border: 1px solid ${p => p.$type === 'local' ? 'rgba(74,222,128,.2)' : 'rgba(139,148,255,.2)'};
  &:hover {
    transform: translateY(-2px);
    box-shadow: ${p => p.$type === 'local'
      ? '0 4px 20px rgba(74,222,128,.15)'
      : '0 4px 20px rgba(139,148,255,.15)'};
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

const AppleIcon = () => (
  <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
    <path d="M18.71 19.5c-.83 1.24-1.71 2.45-3.05 2.47-1.34.03-1.77-.79-3.29-.79-1.53 0-2 .77-3.27.82-1.31.05-2.3-1.32-3.14-2.53C4.25 17 2.94 12.45 4.7 9.39c.87-1.52 2.43-2.48 4.12-2.51 1.28-.02 2.5.87 3.29.87.78 0 2.26-1.07 3.8-.91.65.03 2.47.26 3.64 1.98-.09.06-2.17 1.28-2.15 3.81.03 3.02 2.65 4.03 2.68 4.04-.03.07-.42 1.44-1.38 2.83M13 3.5c.73-.83 1.94-1.46 2.94-1.5.13 1.17-.34 2.35-1.04 3.19-.69.85-1.83 1.51-2.95 1.42-.15-1.15.41-2.35 1.05-3.11z" />
  </svg>
)

const WindowsIcon = () => (
  <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
    <path d="M3 5.557L10.333 4.5v7.056H3V5.557zM11.167 4.371L21 3v8.556h-9.833V4.371zM3 12.444h7.333V19.5L3 18.443v-5.999zM11.167 12.444H21V21l-9.833-1.371v-7.185z" />
  </svg>
)

// ── Page ──────────────────────────────────────────────────────────────────────

function PrixLines({ visible }: { visible: boolean }) {

  return (
    <svg viewBox="0 0 160 12" style={{ position: 'absolute', bottom: '-6px', left: 0, width: '100%', height: '10px', overflow: 'visible', pointerEvents: 'none' }}>
      <path
        d="M 0 9 Q 80 2 160 6"
        fill="none" stroke="rgba(220,38,38,1)" strokeWidth="1.2" strokeLinecap="round"
        pathLength="1"
        style={{
          strokeDasharray: 1,
          strokeDashoffset: visible ? 0 : 1,
          transition: visible ? 'stroke-dashoffset 1.2s ease 0s' : 'none',
        }}
      />
      <path
        d="M 60 13 Q 120 7 160 10"
        fill="none" stroke="rgba(220,38,38,.75)" strokeWidth="1" strokeLinecap="round"
        pathLength="1"
        style={{
          strokeDasharray: 1,
          strokeDashoffset: visible ? 0 : -1,
          transition: visible ? 'stroke-dashoffset 1.0s ease 1.2s' : 'none',
        }}
      />
    </svg>
  )
}

export default function HomePage() {
  const { lang, setLang, t } = useLang()
  const shieldRef = useRef<HTMLDivElement>(null)
  const compareTableRef = useRef<HTMLDivElement>(null)
  const [compareVisible, setCompareVisible] = useState(false)

  useEffect(() => {
    const el = compareTableRef.current
    if (!el) return
    const obs = new IntersectionObserver(
      ([e]) => { if (e.isIntersecting) { setCompareVisible(true); obs.disconnect() } },
      { threshold: 0.85 }
    )
    obs.observe(el)
    return () => obs.disconnect()
  }, [])

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
            <Logo>OSMO<span>zzz</span></Logo>
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
            <DownloadRow>
              <BtnDownload href="https://github.com/platre11/OSMOzzz/releases/latest/download/osmozzz.pkg">
                <AppleIcon />
                {t('heroDownloadMac')}
              </BtnDownload>
              <BtnDownloadWin href="https://github.com/platre11/OSMOzzz/releases/latest/download/osmozzz-setup.exe">
                <WindowsIcon />
                {t('heroDownloadWindows')}
              </BtnDownloadWin>
            </DownloadRow>
            <HeroMeta>{t('heroMeta')}</HeroMeta>
          </HeroActions>
        </HeroSection>
      </HeroWrapper>

      {/* VISION */}
      <Section >
        <Container>
          <VisionLayout>
            <VisionIntro>
              <H2>{t('visionTitle')}</H2>
              {t('visionBody').split('\n\n').map((p, i) => (
                <VisionLead key={i}>{p}</VisionLead>
              ))}
            </VisionIntro>
            <ShieldRadar />
          </VisionLayout>
        </Container>
      </Section>

      {/* COMPARISON */}
      <Section>
        <Container>
          <H2>
            {t('compareTitle').split(/\s+vs\s+/i)[0]}{' '}
            <span style={{
              display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
              fontSize: '0.45em', fontWeight: 800, letterSpacing: '0.08em',
              padding: '3px 10px', borderRadius: '6px',
              position: 'relative', overflow: 'hidden',
              border: 'none',
              verticalAlign: 'middle', margin: '0 6px', top: '-3px',
              color: '#fff',
            }}>
              {/* angle bas-gauche rouge */}
              <span style={{ position: 'absolute', bottom: 0, left: 0, width: 8, height: 8, borderBottom: '1.5px solid rgba(239,68,68,.7)', borderLeft: '1.5px solid rgba(239,68,68,.7)' }} />
              {/* angle haut-droite vert */}
              <span style={{ position: 'absolute', top: 0, right: 0, width: 8, height: 8, borderTop: '1.5px solid rgba(34,197,94,.7)', borderRight: '1.5px solid rgba(34,197,94,.7)' }} />
              <span style={{ position: 'relative', zIndex: 1 }}>VS</span>
            </span>{' '}
            {t('compareTitle').split(/\s+vs\s+/i)[1]}
          </H2>
          <SectionSub>
            {t('compareSub').replace(/[—–]\s*.+$/, '— ')}
            <span style={{ position: 'relative', display: 'inline-block' }}>
              {t('compareSub').match(/[—–]\s*(.+)$/)?.[1]}
              <PrixLines visible={compareVisible} />
            </span>
          </SectionSub>
          <CompareTable ref={compareTableRef}>
            <CompareHeaderRow>
              <CompareHeaderBad>
                <CompareHeaderDot />
                <CompareHeaderLabel>{t('compareWithoutBadge').replace(/\s*\S+$/, '').trim()}</CompareHeaderLabel>
              </CompareHeaderBad>
              <CompareHeaderGood>
                <CompareHeaderDot $good />
                <CompareHeaderLabel $good>{t('compareWithBadge')}</CompareHeaderLabel>
              </CompareHeaderGood>
            </CompareHeaderRow>
            <CompareBody>
              <CompareBadCol>
                {[1,2,3,4,5].map(i => (
                  <CompareRow key={i} $index={i}>
                    <CompareCellBad>
                      <CellIcon>✕</CellIcon>
                      <span>{t(`compareWithout${i}` as any)}</span>
                    </CompareCellBad>
                  </CompareRow>
                ))}
              </CompareBadCol>
              <CompareSnake>
                {[1,2,3,4,5].map(i => (
                  <CompareRow key={i} $index={i} data-snake-row>
                    <CompareCellGood>
                      <CellIcon $good>✓</CellIcon>
                      <span>{t(`compareWith${i}` as any)}</span>
                    </CompareCellGood>
                  </CompareRow>
                ))}
              </CompareSnake>
            </CompareBody>
          </CompareTable>
          <CompareConclusion>
            <p>{t('compareConclusion')}</p>
          </CompareConclusion>
        </Container>
      </Section>

      {/* SOURCES */}
      <Section style={{ position: 'relative', overflow: 'hidden' }}>
        {/* Pill background */}
        <div style={{
          position: 'absolute',
          top: '50%', left: '13%',
          transform: 'translateY(-50%)',
          width: '100%', height: '85%',
          borderRadius: '999px 0 0 999px',
          background: 'linear-gradient(90deg, rgba(91,94,244,.08) 0%, rgba(91,94,244,.03) 60%, transparent 100%)',
          pointerEvents: 'none',
        }} />
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
                {['Chrome','Safari','iMessage','Apple Notes','Calendar','Terminal','Files'].map(name => (
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
                {['Gmail','Notion','GitHub','Linear','Jira','Supabase'].map(name => (
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
            <DownloadRow style={{ marginTop: '32px' }}>
              <BtnDownload href="https://github.com/platre11/OSMOzzz/releases/latest/download/osmozzz.pkg">
                <AppleIcon />
                {t('ctaDownloadMac')}
              </BtnDownload>
              <BtnDownloadWin href="https://github.com/platre11/OSMOzzz/releases/latest/download/osmozzz-setup.exe">
                <WindowsIcon />
                {t('ctaDownloadWindows')}
              </BtnDownloadWin>
            </DownloadRow>
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
          <Logo style={{ fontSize: '14px' }}>OSMO<span>zzz</span></Logo>
          <span style={{ color: '#4b5563', fontSize: '13px' }}>{t('footerLicense')}</span>
          <FooterLink href="https://github.com/platre11/osmozzz" target="_blank" rel="noreferrer">
            GitHub →
          </FooterLink>
        </FooterInner>
      </FooterEl>
    </>
  )
}
