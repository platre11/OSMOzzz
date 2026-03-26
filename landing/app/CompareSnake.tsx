'use client'
import { useRef, useEffect, useState, ReactNode } from 'react'
import styled from 'styled-components'

const Wrap = styled.div`
  position: relative;
  background: rgba(91, 94, 244, 0.03);
`

const SnakeSvg = styled.svg`
  position: absolute;
  top: 0; left: 0;
  width: 100%; height: 100%;
  pointer-events: none;
  overflow: visible;
  z-index: 5;
  -webkit-mask-image: linear-gradient(to right,
    transparent 0%,
    black 6%,
    black 94%,
    transparent 100%
  );
  mask-image: linear-gradient(to right,
    transparent 0%,
    black 6%,
    black 94%,
    transparent 100%
  );
`

const AnimatedPath = styled.path<{ $total: number }>`
  stroke-dasharray: 55 ${p => Math.max(p.$total - 55, 1)};
  stroke-dashoffset: ${p => p.$total};
  animation: snakeTravel ${p => Math.round(p.$total * 0.006)}s linear infinite;
  @keyframes snakeTravel {
    to { stroke-dashoffset: 0; }
  }
`

export default function CompareSnake({ children }: { children: ReactNode }) {
  const wrapRef  = useRef<HTMLDivElement>(null)
  const pathRef  = useRef<SVGPathElement>(null)
  const [d,   setD]   = useState('')
  const [len, setLen] = useState(0)

  useEffect(() => {
    const wrap = wrapRef.current
    if (!wrap) return

    function build() {
      const rows = Array.from(
        wrap!.querySelectorAll('[data-snake-row]')
      ) as HTMLElement[]
      if (!rows.length) return

      const W = wrap!.offsetWidth
      const pts: string[] = []
      let y = 0

      rows.forEach((row, i) => {
        const h = row.offsetHeight
        if (i === 0) pts.push(`M 0 0`)
        if (i % 2 === 0) {
          // gauche → droite, puis descend côté droit
          pts.push(`L ${W} ${y}`, `L ${W} ${y + h}`)
        } else {
          // droite → gauche, puis descend côté gauche
          pts.push(`L 0 ${y}`, `L 0 ${y + h}`)
        }
        y += h
      })

      // Ferme la boucle hors zone (retour vers (0,0) en longeant le bord gauche)
      const lastX = rows.length % 2 === 0 ? W : 0
      pts.push(
        `L ${lastX} ${y}`,
        `L ${lastX} -6`,
        `L 0 -6`,
        `L 0 0`
      )

      const newD = pts.join(' ')
      setD(newD)
      setLen(0) // reset pour forcer remesure

      requestAnimationFrame(() => {
        if (pathRef.current) {
          setLen(pathRef.current.getTotalLength())
        }
      })
    }

    const ro = new ResizeObserver(build)
    ro.observe(wrap)
    build()
    return () => ro.disconnect()
  }, [])

  return (
    <Wrap ref={wrapRef}>
      {children}
      {d && (
        <SnakeSvg>
          <defs>
            <filter id="snake-glow" x="-30%" y="-30%" width="160%" height="160%">
              <feGaussianBlur stdDeviation="2.5" result="blur" />
              <feMerge>
                <feMergeNode in="blur" />
                <feMergeNode in="SourceGraphic" />
              </feMerge>
            </filter>
          </defs>

          {/* Chemin fantôme pour mesurer la longueur */}
          {len === 0 && (
            <path ref={pathRef} d={d} fill="none" stroke="transparent" strokeWidth="1" />
          )}

          {/* Trace de fond (le chemin complet, très subtil) */}
          {len > 0 && (
            <path
              d={d}
              fill="none"
              stroke="rgba(91,94,244,0.12)"
              strokeWidth="0.5"
            />
          )}

          {/* Le serpent animé */}
          {len > 0 && (
            <AnimatedPath
              d={d}
              fill="none"
              stroke="#818cf8"
              strokeWidth="0.7"
              strokeLinecap="round"
              filter="url(#snake-glow)"
              $total={len}
            />
          )}
        </SnakeSvg>
      )}
    </Wrap>
  )
}
