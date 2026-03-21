"use client";

import { motion } from "framer-motion";
import Link from "next/link";

export default function LampCTA() {
  return (
    <section
      className="relative py-32 px-6"
      style={{ background: '#080808' }}
    >
      {/* Subtle radial gold glow — blends into surrounding #080808 */}
      <div
        className="absolute inset-0 pointer-events-none"
        style={{
          background: 'radial-gradient(ellipse 60% 40% at 50% 45%, rgba(201,168,76,0.08) 0%, transparent 70%)',
        }}
      />

      {/* Thin gold accent line */}
      <motion.div
        initial={{ width: 0, opacity: 0 }}
        whileInView={{ width: '12rem', opacity: 1 }}
        transition={{ duration: 0.8, ease: 'easeOut' }}
        viewport={{ once: true }}
        className="mx-auto h-px mb-16"
        style={{ background: 'linear-gradient(90deg, transparent, #c9a84c, transparent)' }}
      />

      <motion.div
        initial={{ opacity: 0, y: 40 }}
        whileInView={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.7, ease: "easeOut" }}
        viewport={{ once: true }}
        className="relative z-10 flex flex-col items-center text-center gap-6 max-w-2xl mx-auto"
      >
        {/* Gold badge */}
        <span
          className="text-xs font-semibold tracking-[0.22em] uppercase px-4 py-1.5 rounded-full"
          style={{
            color: 'rgba(201,168,76,0.8)',
            background: 'rgba(201,168,76,0.06)',
            border: '1px solid rgba(201,168,76,0.2)',
          }}
        >
          Free &amp; Open Source
        </span>

        <h2
          className="text-4xl font-bold text-white sm:text-5xl md:text-6xl leading-tight"
          style={{ letterSpacing: '-0.035em' }}
        >
          Your Documents.{' '}
          <span style={{ color: '#c9a84c' }}>Your AI.</span>{' '}
          Your Device.
        </h2>

        <p
          className="text-base sm:text-lg leading-relaxed"
          style={{ color: 'rgba(255,255,255,0.45)', maxWidth: 440 }}
        >
          Download Justice AI and start researching in minutes.
          <br />
          No signup. No subscription. No compromise.
        </p>

        <div className="flex flex-col sm:flex-row items-center gap-3 mt-2">
          <Link
            href="#download"
            className="gold-solid-btn inline-flex items-center gap-2 px-7 py-3 rounded-xl text-sm font-semibold"
          >
            <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
              <path d="M7.47 10.78a.75.75 0 0 0 1.06 0l3.75-3.75a.75.75 0 0 0-1.06-1.06L8.75 8.44V1.75a.75.75 0 0 0-1.5 0v6.69L4.78 5.97a.75.75 0 0 0-1.06 1.06l3.75 3.75zM3.75 13a.75.75 0 0 0 0 1.5h8.5a.75.75 0 0 0 0-1.5h-8.5z" />
            </svg>
            Download Free
          </Link>
          <a
            href="https://github.com/lastbaa/JusticeAI"
            target="_blank"
            rel="noopener noreferrer"
            className="gold-outline-btn inline-flex items-center gap-2 px-7 py-3 rounded-xl text-sm font-medium"
          >
            <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
              <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0 0 16 8c0-4.42-3.58-8-8-8z" />
            </svg>
            View on GitHub
          </a>
        </div>

        <p className="text-xs" style={{ color: 'rgba(255,255,255,0.3)' }}>
          macOS · Windows · Linux · Open Source · MIT License
        </p>
      </motion.div>
    </section>
  );
}
