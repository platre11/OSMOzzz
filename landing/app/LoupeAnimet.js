"use client";
import React, { useEffect, useRef } from "react";
import styled from "styled-components";

const Screen = styled.div`
  position: fixed;
  top: 5vh;
  left: 50%;
  transform: translateX(-50%);
  width: 1200px;
  height: 90vh;
  overflow: hidden;
  pointer-events: none;
  z-index: 9999;
  contain: layout style paint;
`;

const Cube = styled.div`
  position: absolute;
  top: 0;
  left: 0;
  width: 40px;
  height: 40px;
  background: transparent;
  will-change: transform;
  backface-visibility: hidden;
`;

const CORNER_SIZE = "10px";
const BORDER = "0.5px solid rgba(255, 255, 255, 0.9)";

const CornerTL = styled.div`
  position: absolute;
  top: 0;
  left: 0;
  width: ${CORNER_SIZE};
  height: ${CORNER_SIZE};
  border-top: ${BORDER};
  border-left: ${BORDER};
`;
const CornerTR = styled.div`
  position: absolute;
  top: 0;
  right: 0;
  width: ${CORNER_SIZE};
  height: ${CORNER_SIZE};
  border-top: ${BORDER};
  border-right: ${BORDER};
`;
const CornerBL = styled.div`
  position: absolute;
  bottom: 0;
  left: 0;
  width: ${CORNER_SIZE};
  height: ${CORNER_SIZE};
  border-bottom: ${BORDER};
  border-left: ${BORDER};
`;
const CornerBR = styled.div`
  position: absolute;
  bottom: 0;
  right: 0;
  width: ${CORNER_SIZE};
  height: ${CORNER_SIZE};
  border-bottom: ${BORDER};
  border-right: ${BORDER};
`;

// idle: 'none' | 'grow' | 'shrink' | 'pulse-grow' | 'pulse-shrink' | 'rotate'
// rotate utilisé rarement (2 fois sur 12)
const SEQUENCES = [
  {
    dx: -0.25,
    dy: -0.2,
    scale: 1.0,
    moveDuration: 900,
    pauseDuration: 1000,
    idle: "none",
  },
  {
    dx: 0.3,
    dy: 0.25,
    scale: 0.85,
    moveDuration: 1100,
    pauseDuration: 1000,
    idle: "grow",
  },
  {
    dx: -0.05,
    dy: -0.05,
    scale: 1.0,
    moveDuration: 800,
    pauseDuration: 1200,
    idle: "none",
  },
  {
    dx: 0.2,
    dy: -0.35,
    scale: 0.9,
    moveDuration: 1000,
    pauseDuration: 700,
    idle: "shrink",
  },
  {
    dx: 0.02,
    dy: 0.02,
    scale: 1.0,
    moveDuration: 600,
    pauseDuration: 1500,
    idle: "none",
  },
  {
    dx: 0.1,
    dy: -0.2,
    scale: 0.8,
    moveDuration: 700,
    pauseDuration: 1100,
    idle: "pulse-grow",
  },
  {
    dx: -0.15,
    dy: 0.3,
    scale: 1.1,
    moveDuration: 800,
    pauseDuration: 1000,
    idle: "none",
  },
  {
    dx: 0.25,
    dy: 0.15,
    scale: 1.0,
    moveDuration: 950,
    pauseDuration: 800,
    idle: "rotate",
  },
  {
    dx: -0.02,
    dy: -0.02,
    scale: 1.0,
    moveDuration: 500,
    pauseDuration: 1800,
    idle: "none",
  },
  {
    dx: -0.2,
    dy: -0.3,
    scale: 0.9,
    moveDuration: 850,
    pauseDuration: 600,
    idle: "pulse-shrink",
  },
  {
    dx: 0.15,
    dy: 0.35,
    scale: 1.15,
    moveDuration: 1050,
    pauseDuration: 1000,
    idle: "none",
  },
  {
    dx: 0.2,
    dy: -0.1,
    scale: 1.1,
    moveDuration: 880,
    pauseDuration: 900,
    idle: "grow",
  },
  {
    dx: 0.02,
    dy: -0.02,
    scale: 1.0,
    moveDuration: 600,
    pauseDuration: 1200,
    idle: "none",
  },
  {
    dx: -0.25,
    dy: 0.2,
    scale: 0.85,
    moveDuration: 1000,
    pauseDuration: 800,
    idle: "rotate",
  },
  {
    dx: -0.1,
    dy: -0.15,
    scale: 0.75,
    moveDuration: 750,
    pauseDuration: 1000,
    idle: "none",
  },
];

const clamp = (v, min, max) => Math.max(min, Math.min(v, max));
const easeInOut = (t) => (t < 0.5 ? 2 * t * t : -1 + (4 - 2 * t) * t);
const sleep = (ms) => new Promise((res) => setTimeout(res, ms));

const LoupeAnimet = () => {
  const cubeRef = useRef(null);
  const screenRef = useRef(null);

  useEffect(() => {
    const cube = cubeRef.current;
    const screen = screenRef.current;
    if (!cube || !screen) return;

    const SIZE = 40;
    let W = screen.offsetWidth;
    let H = screen.offsetHeight;
    const getW = () => W;
    const getH = () => H;

    const ro = new ResizeObserver((entries) => {
      const r = entries[0].contentRect;
      W = r.width;
      H = r.height;
    });
    ro.observe(screen);

    let pos = { x: W * 0.5, y: H * 0.5 };
    let currentScale = 1;
    let currentRotation = 0;
    let seqIndex = 0;
    let cancelled = false;

    const applyTransform = () => {
      cube.style.transform = `translate3d(${pos.x}px, ${pos.y}px, 0) rotate(${currentRotation}deg) scale(${currentScale})`;
    };

    const moveTo = (targetX, targetY, targetScale, duration) =>
      new Promise((res) => {
        const startX = pos.x,
          startY = pos.y,
          startScale = currentScale;
        const start = performance.now();
        const frame = (now) => {
          if (cancelled) return res();
          const t = Math.min((now - start) / duration, 1);
          const e = easeInOut(t);
          pos.x = startX + (targetX - startX) * e;
          pos.y = startY + (targetY - startY) * e;
          currentScale = startScale + (targetScale - startScale) * e;
          applyTransform();
          t < 1 ? requestAnimationFrame(frame) : res();
        };
        requestAnimationFrame(frame);
      });

    // Grossit progressivement vers targetScale et reste
    const animateScale = (targetScale, duration) =>
      new Promise((res) => {
        const startScale = currentScale;
        const start = performance.now();
        const frame = (now) => {
          if (cancelled) return res();
          const t = Math.min((now - start) / duration, 1);
          currentScale = startScale + (targetScale - startScale) * easeInOut(t);
          applyTransform();
          t < 1 ? requestAnimationFrame(frame) : res();
        };
        requestAnimationFrame(frame);
      });

    // Pulse : va vers peak puis revient à la base
    const doPulse = (peak, duration) =>
      new Promise((res) => {
        const base = currentScale;
        const start = performance.now();
        const frame = (now) => {
          if (cancelled) return res();
          const t = Math.min((now - start) / duration, 1);
          currentScale = base + (peak - base) * Math.sin(t * Math.PI);
          applyTransform();
          if (t < 1) requestAnimationFrame(frame);
          else {
            currentScale = base;
            applyTransform();
            res();
          }
        };
        requestAnimationFrame(frame);
      });

    const doRotate = (duration) =>
      new Promise((res) => {
        const startRot = currentRotation;
        const targetRot = currentRotation + 90;
        const start = performance.now();
        const frame = (now) => {
          if (cancelled) return res();
          const t = Math.min((now - start) / duration, 1);
          currentRotation = startRot + (targetRot - startRot) * easeInOut(t);
          applyTransform();
          t < 1 ? requestAnimationFrame(frame) : res();
        };
        requestAnimationFrame(frame);
      });

    const doIdle = async (idle, duration) => {
      const base = currentScale;
      switch (idle) {
        case "grow":
          await animateScale(base * 1.6, duration);
          break;
        case "shrink":
          await animateScale(base * 0.5, duration);
          break;
        case "pulse-grow":
          await doPulse(base * 1.7, duration);
          break;
        case "pulse-shrink":
          await doPulse(base * 0.45, duration);
          break;
        case "rotate":
          await doRotate(duration);
          break;
        default:
          await sleep(duration);
          break;
      }
    };

    const EDGE_MARGIN = 60;
    const isNearEdge = () => {
      const W = getW(), H = getH();
      return (
        pos.x < EDGE_MARGIN || pos.x > W - SIZE - EDGE_MARGIN ||
        pos.y < EDGE_MARGIN || pos.y > H - SIZE - EDGE_MARGIN
      );
    };

    const MARGIN = 20;

    const run = async () => {
      while (!cancelled) {
        const seq = SEQUENCES[seqIndex % SEQUENCES.length];
        const W = getW(), H = getH();
        const targetX = clamp(pos.x + seq.dx * W, MARGIN, W - SIZE - MARGIN);
        const targetY = clamp(pos.y + seq.dy * H, MARGIN, H - SIZE - MARGIN);

        await moveTo(targetX, targetY, seq.scale, seq.moveDuration);
        const idle = isNearEdge() ? 'none' : seq.idle;
        await doIdle(idle, seq.pauseDuration);
        seqIndex++;
      }
    };

    run();
    return () => {
      cancelled = true;
      ro.disconnect();
    };
  }, []);

  return (
    <Screen ref={screenRef}>
      <Cube ref={cubeRef}>
        <CornerTL />
        <CornerTR />
        <CornerBL />
        <CornerBR />
      </Cube>
    </Screen>
  );
};

export default LoupeAnimet;
