"use client";
import React from "react";
import styled, { keyframes } from "styled-components";

// ── Animations ────────────────────────────────────────────────────────────────

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

// ── Styled ────────────────────────────────────────────────────────────────────

const SweepG = styled.g`
  transform-origin: 130px 130px;
  animation: ${rotateSweep} 6s linear infinite;
`;

const BlipDot = styled.circle`
  animation: ${blipFlash} ${p => p.$dur} ease-in-out ${p => p.$delay} infinite;
`;

const RippleRing = styled.circle`
  fill: none;
  stroke: #5b5ef4;
  animation: ${ripple} ${p => p.$dur} ease-out ${p => p.$delay} infinite;
`;

// ── Blips ─────────────────────────────────────────────────────────────────────

const toXY = (angleDeg, r) => ({
  x: +(130 + r * Math.sin((angleDeg * Math.PI) / 180)).toFixed(1),
  y: +(130 - r * Math.cos((angleDeg * Math.PI) / 180)).toFixed(1),
});

const BLIPS = [
  { ...toXY(52,  58),  delay: "0.4s", dur: "4.2s", ripple: true,  rippleDur: "2s",   rippleDelay: "0.6s" },
  { ...toXY(135, 86),  delay: "2.1s", dur: "5.0s", ripple: false },
  { ...toXY(205, 42),  delay: "1.0s", dur: "3.8s", ripple: false },
  { ...toXY(295, 112), delay: "3.2s", dur: "4.5s", ripple: true,  rippleDur: "2.2s", rippleDelay: "3.4s" },
  { ...toXY(165, 72),  delay: "1.8s", dur: "4.0s", ripple: false },
];

// ── Component ─────────────────────────────────────────────────────────────────

export default function ShieldRadar() {
  return (
    <svg
      viewBox="0 0 260 260"
      width="260"
      height="260"
      style={{ overflow: "visible", flexShrink: 0 }}
    >
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
        {/* Masque de fondu pour le fond du radar */}
        <radialGradient id="sr-fade" cx="50%" cy="50%" r="50%">
          <stop offset="60%" stopColor="rgba(91,94,244,0.06)" />
          <stop offset="100%" stopColor="rgba(91,94,244,0)" />
        </radialGradient>
      </defs>

      {/* Anneaux décoratifs extérieurs — débordent hors du SVG */}
      <circle cx="130" cy="130" r="135" fill="none" stroke="rgba(255,255,255,0.055)" strokeWidth="0.6" />
      <circle cx="130" cy="130" r="152" fill="none" stroke="rgba(255,255,255,0.045)" strokeWidth="0.6" />
      <circle cx="130" cy="130" r="170" fill="none" stroke="rgba(255,255,255,0.035)" strokeWidth="0.5" />
      <circle cx="130" cy="130" r="190" fill="none" stroke="rgba(255,255,255,0.025)" strokeWidth="0.5" />
      <circle cx="130" cy="130" r="212" fill="none" stroke="rgba(255,255,255,0.015)" strokeWidth="0.4" />

      {/* Fond dégradé (sans bordure dure) */}
      <circle cx="130" cy="130" r="120" fill="url(#sr-fade)" />

      <g clipPath="url(#sr-clip)">
        {/* Anneaux internes */}
        <circle cx="130" cy="130" r="30"  fill="none" stroke="rgba(255,255,255,0.07)" strokeWidth="0.7" />
        <circle cx="130" cy="130" r="58"  fill="none" stroke="rgba(255,255,255,0.06)" strokeWidth="0.7" />
        <circle cx="130" cy="130" r="86"  fill="none" stroke="rgba(255,255,255,0.05)" strokeWidth="0.7" />
        <circle cx="130" cy="130" r="114" fill="none" stroke="rgba(255,255,255,0.04)" strokeWidth="0.7" />

        {/* Balayage radar + traînée phosphore */}
        <SweepG>
          <path d="M130,130 L26,70  A120,120 0 0,1 61,32  Z" fill="rgba(91,94,244,0.04)" />
          <path d="M130,130 L61,32  A120,120 0 0,1 99,14  Z" fill="rgba(91,94,244,0.08)" />
          <path d="M130,130 L99,14  A120,120 0 0,1 130,10 Z" fill="rgba(91,94,244,0.14)" />
          <path d="M130,130 L130,10 A120,120 0 0,1 190,26 Z" fill="rgba(91,94,244,0.18)" />
          <line x1="130" y1="130" x2="130" y2="10" stroke="#5b5ef4" strokeWidth="1" opacity="0.8" />
        </SweepG>

        {/* Blips */}
        {BLIPS.map((b, i) => (
          <g key={i}>
            <BlipDot cx={b.x} cy={b.y} r="2.5" fill="#5b5ef4" $dur={b.dur} $delay={b.delay} />
            {b.ripple && (
              <RippleRing cx={b.x} cy={b.y} r="3" $dur={b.rippleDur} $delay={b.rippleDelay} />
            )}
          </g>
        ))}

        {/* Point central */}
        <circle cx="130" cy="130" r="3" fill="#5b5ef4" opacity="0.9" />
        <circle cx="130" cy="130" r="7" fill="none" stroke="#5b5ef4" strokeWidth="0.5" opacity="0.35" />
      </g>

      {/* Lueur subtile sur le bord du radar (pas de bordure dure) */}
      <circle cx="130" cy="130" r="120" fill="none" stroke="rgba(91,94,244,0.2)" strokeWidth="0.8" filter="url(#sr-glow)" />
    </svg>
  );
}
