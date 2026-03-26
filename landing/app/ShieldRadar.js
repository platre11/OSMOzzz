"use client";
import React, { useRef, useEffect, useState } from "react";
import styled, { keyframes, css } from "styled-components";

// ── Keyframes ──────────────────────────────────────────────────────────────────

const rotateSweep = keyframes`
  from { transform: rotate(0deg); }
  to   { transform: rotate(360deg); }
`;

const blipFlash = keyframes`
  0%, 100% { opacity: 0; }
  10%       { opacity: 1; }
  55%       { opacity: 0.7; }
  90%       { opacity: 0; }
`;

const ripple = keyframes`
  0%   { r: 3;  opacity: 0.8; stroke-width: 1; }
  100% { r: 16; opacity: 0;   stroke-width: 0.3; }
`;

// Apparition d'un anneau : expanse depuis le centre + fade in
const ringAppear = keyframes`
  0%   { opacity: 0; transform: scale(0.1); }
  70%  { opacity: 1; transform: scale(1.06); }
  100% { opacity: 1; transform: scale(1); }
`;

// Point central : pop
const dotPop = keyframes`
  0%   { opacity: 0; transform: scale(0); }
  60%  { opacity: 1; transform: scale(1.5); }
  100% { opacity: 0.9; transform: scale(1); }
`;

// Balayage : fade in depuis 0
const sweepFadeIn = keyframes`
  0%   { opacity: 0; }
  100% { opacity: 1; }
`;

// Anneau extérieur : slide depuis l'extérieur
const outerRingFade = keyframes`
  0%   { opacity: 0; transform: scale(1.15); }
  100% { opacity: 1; transform: scale(1); }
`;

// ── Styled ─────────────────────────────────────────────────────────────────────

const RadarWrap = styled.div`
  position: relative;
  display: inline-block;
`;

// Anneau interne animé séquentiellement
const AnimRing = styled.circle`
  transform-origin: 130px 130px;
  opacity: 0;
  ${p => p.$visible && css`
    animation: ${ringAppear} 0.7s cubic-bezier(0.22, 1, 0.36, 1) ${p.$delay} both;
  `}
`;

// Anneaux extérieurs décoratifs
const OuterRing = styled.circle`
  transform-origin: 130px 130px;
  opacity: 0;
  ${p => p.$visible && css`
    animation: ${outerRingFade} 1s ease-out ${p.$delay} both;
  `}
`;

// Fond dégradé
const FadeCircle = styled.circle`
  opacity: 0;
  ${p => p.$visible && css`
    animation: ${sweepFadeIn} 0.8s ease-out ${p.$delay} both;
  `}
`;

// Balayage
const SweepG = styled.g`
  transform-origin: 130px 130px;
  animation: ${rotateSweep} 6s linear infinite;
  opacity: 0;
  ${p => p.$visible && css`
    animation:
      ${rotateSweep} 6s linear ${p.$delay} infinite,
      ${sweepFadeIn} 0.6s ease-out ${p.$delay} both;
  `}
`;

// Point central
const CenterDot = styled.circle`
  transform-origin: 130px 130px;
  opacity: 0;
  ${p => p.$visible && css`
    animation: ${dotPop} 0.5s cubic-bezier(0.22, 1, 0.36, 1) ${p.$delay} both;
  `}
`;

const CenterRing = styled.circle`
  transform-origin: 130px 130px;
  opacity: 0;
  ${p => p.$visible && css`
    animation: ${ringAppear} 0.6s cubic-bezier(0.22, 1, 0.36, 1) ${p.$delay} both;
  `}
`;

// Blips — n'apparaissent qu'après le sweep
const BlipDot = styled.circle`
  opacity: 0;
  ${p => p.$visible && css`
    animation: ${blipFlash} ${p.$dur} ease-in-out ${p.$blipDelay} infinite;
  `}
`;

const RippleRing = styled.circle`
  fill: none;
  stroke: #5b5ef4;
  opacity: 0;
  ${p => p.$visible && css`
    animation: ${ripple} ${p.$dur} ease-out ${p.$delay} infinite;
  `}
`;

// Lueur bord
const GlowRing = styled.circle`
  opacity: 0;
  ${p => p.$visible && css`
    animation: ${sweepFadeIn} 1s ease-out ${p.$delay} both;
  `}
`;

// ── Blips ──────────────────────────────────────────────────────────────────────

const toXY = (angleDeg, r) => ({
  x: +(130 + r * Math.sin((angleDeg * Math.PI) / 180)).toFixed(1),
  y: +(130 - r * Math.cos((angleDeg * Math.PI) / 180)).toFixed(1),
});

const BLIPS = [
  { ...toXY(52,  58),  dur: "4.2s", blipDelay: "2.0s", ripple: true,  rippleDur: "2s",   rippleDelay: "2.2s" },
  { ...toXY(135, 86),  dur: "5.0s", blipDelay: "2.8s", ripple: false },
  { ...toXY(205, 42),  dur: "3.8s", blipDelay: "2.4s", ripple: false },
  { ...toXY(295, 112), dur: "4.5s", blipDelay: "3.4s", ripple: true,  rippleDur: "2.2s", rippleDelay: "3.6s" },
  { ...toXY(165, 72),  dur: "4.0s", blipDelay: "3.0s", ripple: false },
];

// ── Component ──────────────────────────────────────────────────────────────────

export default function ShieldRadar() {
  const ref   = useRef(null);
  const [vis, setVis] = useState(false);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const io = new IntersectionObserver(
      ([e]) => { if (e.isIntersecting) { setVis(true); io.disconnect(); } },
      { threshold: 0.3 }
    );
    io.observe(el);
    return () => io.disconnect();
  }, []);

  return (
    <RadarWrap ref={ref}>
      <svg viewBox="0 0 260 260" width="260" height="260" style={{ overflow: "visible", flexShrink: 0 }}>
        <defs>
          <clipPath id="sr-clip">
            <circle cx="130" cy="130" r="120" />
          </clipPath>
          <filter id="sr-glow" x="-20%" y="-20%" width="140%" height="140%">
            <feGaussianBlur stdDeviation="4" result="blur" />
            <feMerge>
              <feMergeNode in="blur" />
              <feMergeNode in="SourceGraphic" />
            </feMerge>
          </filter>
          <radialGradient id="sr-fade" cx="50%" cy="50%" r="50%">
            <stop offset="60%" stopColor="rgba(91,94,244,0.06)" />
            <stop offset="100%" stopColor="rgba(91,94,244,0)" />
          </radialGradient>
        </defs>

        {/* 1. Anneaux extérieurs décoratifs — décalés un par un */}
        <OuterRing cx="130" cy="130" r="135" fill="none" stroke="rgba(255,255,255,0.055)" strokeWidth="0.6" $visible={vis} $delay="0.1s" />
        <OuterRing cx="130" cy="130" r="152" fill="none" stroke="rgba(255,255,255,0.045)" strokeWidth="0.6" $visible={vis} $delay="0.25s" />
        <OuterRing cx="130" cy="130" r="170" fill="none" stroke="rgba(255,255,255,0.035)" strokeWidth="0.5" $visible={vis} $delay="0.4s" />
        <OuterRing cx="130" cy="130" r="190" fill="none" stroke="rgba(255,255,255,0.025)" strokeWidth="0.5" $visible={vis} $delay="0.55s" />
        <OuterRing cx="130" cy="130" r="212" fill="none" stroke="rgba(255,255,255,0.015)" strokeWidth="0.4" $visible={vis} $delay="0.7s" />

        {/* 2. Fond dégradé */}
        <FadeCircle cx="130" cy="130" r="120" fill="url(#sr-fade)" $visible={vis} $delay="0.3s" />

        <g clipPath="url(#sr-clip)">
          {/* 3. Anneaux internes — du centre vers l'extérieur */}
          <AnimRing cx="130" cy="130" r="30"  fill="none" stroke="rgba(255,255,255,0.07)" strokeWidth="0.7" $visible={vis} $delay="0.2s" />
          <AnimRing cx="130" cy="130" r="58"  fill="none" stroke="rgba(255,255,255,0.06)" strokeWidth="0.7" $visible={vis} $delay="0.45s" />
          <AnimRing cx="130" cy="130" r="86"  fill="none" stroke="rgba(255,255,255,0.05)" strokeWidth="0.7" $visible={vis} $delay="0.7s" />
          <AnimRing cx="130" cy="130" r="114" fill="none" stroke="rgba(255,255,255,0.04)" strokeWidth="0.7" $visible={vis} $delay="0.95s" />

          {/* 4. Balayage — démarre après les anneaux */}
          <SweepG $visible={vis} $delay="1.2s">
            <path d="M130,130 L26,70  A120,120 0 0,1 61,32  Z" fill="rgba(91,94,244,0.04)" />
            <path d="M130,130 L61,32  A120,120 0 0,1 99,14  Z" fill="rgba(91,94,244,0.08)" />
            <path d="M130,130 L99,14  A120,120 0 0,1 130,10 Z" fill="rgba(91,94,244,0.14)" />
            <path d="M130,130 L130,10 A120,120 0 0,1 190,26 Z" fill="rgba(91,94,244,0.18)" />
            <line x1="130" y1="130" x2="130" y2="10" stroke="#5b5ef4" strokeWidth="1" opacity="0.8" />
          </SweepG>

          {/* 5. Blips — apparaissent après le premier tour de sweep */}
          {BLIPS.map((b, i) => (
            <g key={i}>
              <BlipDot cx={b.x} cy={b.y} r="2.5" fill="#5b5ef4" $visible={vis} $dur={b.dur} $blipDelay={b.blipDelay} />
              {b.ripple && (
                <RippleRing cx={b.x} cy={b.y} r="3" $visible={vis} $dur={b.rippleDur} $delay={b.rippleDelay} />
              )}
            </g>
          ))}

          {/* 6. Point central — pop en dernier */}
          <CenterDot cx="130" cy="130" r="3"  fill="#5b5ef4"  $visible={vis} $delay="1.0s" />
          <CenterRing cx="130" cy="130" r="7"  fill="none" stroke="#5b5ef4" strokeWidth="0.5" opacity="0.35" $visible={vis} $delay="1.05s" />
        </g>

        {/* 7. Lueur bord */}
        <GlowRing cx="130" cy="130" r="120" fill="none" stroke="rgba(91,94,244,0.2)" strokeWidth="0.8" filter="url(#sr-glow)" $visible={vis} $delay="1.1s" />
      </svg>
    </RadarWrap>
  );
}
