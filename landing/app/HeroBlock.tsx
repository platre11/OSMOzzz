'use client'
import { useEffect, useRef, useState } from 'react'
import styled, { keyframes, css } from 'styled-components'
import { useLang } from '../context/LanguageContext'

// ── Styled ────────────────────────────────────────────────────────────────────

const Wrapper = styled.div`
  position: relative;
  width: 100%;
  max-width: 1000px;
  height: 480px;
  margin: 0 auto;
  padding: 0 24px;
  overflow: hidden;
`

const Side = styled.div`
  position: absolute;
  top: 0;
  left: 0;
  display: flex;
  flex-direction: column;
  gap: 16px;
  width: 380px;
  z-index: 10;
`

const canvasEntrance = keyframes`
  0%   { transform: translateX(-50%) translateY(calc(-50% + 28px)); opacity: 0; }
  65%  { transform: translateX(-50%) translateY(calc(-50% - 10px)); opacity: 1; }
  100% { transform: translateX(-50%) translateY(-50%);              opacity: 1; }
`

const CanvasWrapper = styled.div<{ $visible: boolean; $top: string; $left: string }>`
  position: absolute;
  z-index: 5;
  width: 380px;
  height: 380px;
  top: ${p => p.$top};
  left: ${p => p.$left};
  opacity: 0;
  pointer-events: none;
  canvas { display: block; }
  ${p => p.$visible && css`
    animation: ${canvasEntrance} 0.9s cubic-bezier(0.4, 0, 0.2, 1) 0.1s forwards;
  `}
`


const Label = styled.p`
  font-size: 13px;
  color: #6b7280;
  font-weight: 500;
`

const InputRow = styled.div`
  display: flex;
  gap: 10px;
`

const Input = styled.input`
  flex: 1;
  background: #0f1117;
  border: 1px solid #1f2230;
  border-radius: 10px;
  padding: 12px 16px;
  font-size: 14px;
  color: #e8eaf0;
  outline: none;
  font-family: inherit;
  transition: border-color .15s;
  &::placeholder { color: #4b5563; }
  &:focus { border-color: #5b5ef4; }
`

const btnExitAnim = keyframes`
  0%   { transform: scale(1);     opacity: 1; }
  15%  { transform: scale(0.88);  opacity: 1; }
  30%  { transform: scale(1);     opacity: 1; }
  100% { transform: translateX(80px) scale(1); opacity: 0; }
`

const Btn = styled.button<{ $exit?: boolean }>`
  padding: 12px 20px;
  border-radius: 10px;
  background: #5b5ef4;
  color: #fff;
  border: none;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  font-family: inherit;
  transition: background .15s;
  &:hover { background: #4a4de3; }
  ${p => p.$exit && css`
    animation: ${btnExitAnim} 0.6s ease forwards;
    pointer-events: none;
  `}
`

const growDown = keyframes`
  from { transform: scaleY(0); }
  to   { transform: scaleY(1); }
`

const growRight = keyframes`
  from { transform: scaleX(0); }
  to   { transform: scaleX(1); }
`

const ContainerLine = styled.div<{ $slide: boolean }>`
  position: absolute;
  top: 100px;
  left: 50px;
  width: 0;
  height: 150px;
  border-right: 1px dashed white;
  z-index: 1;
  transform: scaleY(0);
  transform-origin: top;
  ${p => p.$slide && css`
    animation: ${growDown} 0.5s linear forwards;
  `}
`

const Line1 = styled.div<{ $slide: boolean }>`
  position: absolute;
  bottom: 0px;
  left: 0px;
  width: 250px;
  height: 0;
  border-top: 1px dashed white;
  transform: scaleX(0);
  transform-origin: left;
  ${p => p.$slide && css`
    animation: ${growRight} 0.5s linear 0.5s forwards;
  `}
`

const Caption = styled.div<{ $visible: boolean }>`
  position: absolute;
  /* Hex_center est en world (0, 0.6, 0), canvas 380×380 centré dans Wrapper.
     FOV 45°, cam Z=5 → Y=0.6 ≈ 27% au-dessus du centre canvas → ~50px vers le haut.
     On se place juste à droite et au-dessus du hex. */
  left: calc(50% + 15px);
  top: calc(50% - 175px);
  width: 400px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  opacity: ${p => p.$visible ? 1 : 0};
  transition: opacity 0.8s ease 0.2s;
  pointer-events: none;
`

const CaptionLabel = styled.p`
  font-size: 13px;
  color: #e8eaf0;
  line-height: 1.6;
  font-weight: 400;
  margin: 0;
  padding-left: 4px;
`

const CaptionText = styled.div`
  background: #0f1117;
  border: 1px solid #1f2230;
  border-radius: 10px;
  /* décale vers la gauche pour que l'hexagone chevauche le bord gauche du box */
  margin-left: -52px;
  padding: 12px 16px 12px 68px;
  font-size: 14px;
  color: #e8eaf0;
  font-family: inherit;
  line-height: 1.5;
`

const GreenTag = styled.span`
  color: #22c55e;
  font-weight: 600;
  &::before { content: '['; }
  &::after  { content: ']'; }
`

const FinalTextContainer = styled.div<{ $visible: boolean }>`
  position: absolute;
  left: calc(50% + 207px);
  top: calc(50% - 144px);
  width: 150px;
  height: 200px;
  overflow: hidden;
  opacity: ${p => p.$visible ? 1 : 0};
  transition: opacity 0.8s ease 0.2s;
  pointer-events: none;
`

const LineFinal2 = styled.div<{ $slide: boolean }>`
  position: absolute;
  bottom: 0px;
  left: 50px;
  width: 0;
  height: 70px;
  border-right: 1px dashed white;
  transform: scaleY(0);
  transform-origin: top;
  ${p => p.$slide && css`
    animation: ${growDown} 0.5s linear forwards;
  `}
`

const AliasCaption = styled.div<{ $visible: boolean }>`
  position: absolute;
  left: calc(50% + 207px);
  top: calc(50% - 144px + 200px + 12px);
  width: 200px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  opacity: ${p => p.$visible ? 1 : 0};
  transition: opacity 0.8s ease 0.2s;
  pointer-events: none;
`

const AliasInputBox = styled.div`
  background: #0f1117;
  border: 1px solid #1f2230;
  border-radius: 10px;
  padding: 12px 16px;
  font-size: 14px;
  color: #e8eaf0;
  font-family: inherit;
  line-height: 1.5;
`

const AliasTag = styled.span`
  color: #22c55e;
  font-weight: 600;
`

// ── Component ─────────────────────────────────────────────────────────────────

export default function HeroBlock() {
  const wrapperRef = useRef<HTMLDivElement>(null)
  const sideRef    = useRef<HTMLDivElement>(null)
  const canvasRef  = useRef<HTMLDivElement>(null)
  const { t, ready } = useLang()
  const [value, setValue] = useState('')
  const [phase, setPhase] = useState(false)
  const [sliding, setSliding] = useState(false)
  const [typed, setTyped] = useState('')
  const [canvasTop, setCanvasTop]   = useState('50%')
  const [canvasLeft, setCanvasLeft] = useState('50%')
  const [btnExit, setBtnExit] = useState(false)
  const [showCaption, setShowCaption] = useState(false)
  const [showFinal, setShowFinal] = useState(false)
  const [showAlias, setShowAlias] = useState(false)

  useEffect(() => {
    const wrapper = wrapperRef.current
    const side    = sideRef.current
    if (!wrapper || !side) return

    // Centre Side sans transition
    const dx = wrapper.offsetWidth  / 2 - side.offsetWidth  / 2
    const dy = wrapper.offsetHeight / 2 - side.offsetHeight / 2
    side.style.transition = 'none'
    side.style.transform  = `translate(${dx}px, ${dy}px)`

    // Active la transition au prochain frame (0.5s = 2x plus rapide)
    requestAnimationFrame(() => {
      side.style.transition = 'transform 0.5s linear'
    })

    // Typewriter — démarre seulement quand la langue est détectée
    if (!ready) return
    const text = t('heroTypewriter')
    let i = 0
    const typeTimer = setTimeout(() => {
      const interval = setInterval(() => {
        i++
        setTyped(text.slice(0, i))
        if (i >= text.length) clearInterval(interval)
      }, 65)
      timers.push(interval as unknown as ReturnType<typeof setTimeout>)
    }, 400)

    // Bouton : animation click + exit dès que le texte est fini
    const btnTimer = setTimeout(() => setBtnExit(true), 400 + text.length * 65)

    // Déplace Side après que le texte soit écrit + 600ms de pause
    const moveDelay = 400 + text.length * 65 + 600
    const moveTimer = setTimeout(() => {
      side.style.transition = 'transform 0.5s linear'
      side.style.transform = 'translate(0px, 0px)'
      // ContainerLine démarre quand Side finit (0.5s)
      const slideTimer = setTimeout(() => {
        setSliding(true)
        // 3D apparaît quand LibneBackgroundBlack commence à partir vers la droite (50% de 1s)
        const show3d = setTimeout(() => setPhase(true), 0)
        timers.push(show3d)
      }, 500)
      timers.push(slideTimer)
    }, moveDelay)

    const timers: ReturnType<typeof setTimeout>[] = [typeTimer, btnTimer, moveTimer]
    return () => timers.forEach(clearTimeout)
  }, [ready])

  // Charge Three.js seulement quand le canvas devient visible
  useEffect(() => {
    if (!phase || !canvasRef.current) return
    let cleanup: (() => void) | undefined
    import('./shield3d').then(({ initShield }) => {
      if (!canvasRef.current) return
      cleanup = initShield(canvasRef.current)
    })
    const t1 = setTimeout(() => setShowCaption(true), 4100)
    const t2 = setTimeout(() => setShowFinal(true),   4800)
    const t3 = setTimeout(() => setShowAlias(true),   5500)
    return () => { cleanup?.(); clearTimeout(t1); clearTimeout(t2); clearTimeout(t3) }
  }, [phase])

  return (
    <Wrapper ref={wrapperRef}>
      <Side ref={sideRef}>
        <Label>{t('heroInputLabel')}</Label>
        <InputRow>
          <Input
            value={typed || value}
            onChange={e => setValue(e.target.value)}
            placeholder={t('heroInputPlaceholder')}
            readOnly={!!typed && !phase}
          />
          <Btn $exit={btnExit}>→</Btn>
        </InputRow>
      </Side>
      <ContainerLine $slide={sliding}>
        <Line1 $slide={sliding} />
      </ContainerLine>

      <CanvasWrapper $visible={phase} $top={canvasTop} $left={canvasLeft} ref={canvasRef} />

      <Caption $visible={showCaption}>
        <Label>OSMOzzz</Label>
        <CaptionText>
          <GreenTag>OSMOzzz</GreenTag> {t('heroCaptionLine1')}<br />{t('heroCaptionLine2')}
        </CaptionText>
        <CaptionLabel>{t('heroCaptionBottom1')}</CaptionLabel>
      </Caption>
      <FinalTextContainer $visible={showFinal}>
        <LineFinal2 $slide={showFinal} />
      </FinalTextContainer>

      <AliasCaption $visible={showAlias}>
        <Label>{t('heroResponseLabel')}</Label>
        <AliasInputBox>
          <AliasTag>Z8xJNaS7f82</AliasTag> {t('heroCaptionLine1')}<br />{t('heroCaptionLine2')}
        </AliasInputBox>
      </AliasCaption>
    </Wrapper>
  )
}
