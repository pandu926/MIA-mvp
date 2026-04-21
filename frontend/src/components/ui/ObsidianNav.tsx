'use client';

import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { FaArrowRight } from 'react-icons/fa6';

const LINKS = [
  { href: '/app', label: 'Discover' },
  { href: '/mia/runs', label: 'Runs' },
  { href: '/mia/watchlist', label: 'Watch' },
  { href: '/mia/missions', label: 'Missions' },
  { href: '/backtesting', label: 'Proof' },
];

export function ObsidianNav() {
  const pathname = usePathname();
  const primaryAction =
    pathname === '/mia' || pathname.startsWith('/mia?')
      ? { href: '/mia/runs', label: 'Open Runs' }
      : { href: '/mia', label: 'Open Investigation' };

  return (
    <header className="top-nav">
      <div className="mx-auto flex h-full w-full max-w-7xl items-center justify-between px-4 md:px-6">
        <div className="flex items-center gap-3">
          <Link href="/" className="font-headline text-xl font-extrabold tracking-tight" style={{ color: 'var(--primary)' }}>
            MIA
          </Link>
          <span
            className="hidden rounded-lg px-2 py-1 text-[10px] font-bold uppercase tracking-[0.18em] md:inline-flex"
            style={{ color: 'var(--secondary-container)', background: 'rgba(0,255,163,0.08)' }}
          >
            Early Agentic OS
          </span>
        </div>

        <nav className="hidden items-center gap-1 md:flex">
          {LINKS.map((link) => {
            const active = pathname === link.href || pathname.startsWith(`${link.href}/`);
            return (
              <Link
                key={link.href}
                href={link.href}
                data-testid={`obsidian-nav-link-${link.label.toLowerCase()}`}
                className="rounded px-3 py-2 text-sm font-semibold transition-colors"
                style={{
                  color: active ? 'var(--primary)' : 'var(--on-surface-variant)',
                  background: active ? 'rgba(105,137,255,0.12)' : 'transparent',
                }}
              >
                {link.label}
              </Link>
            );
          })}
        </nav>

        <Link
          href={primaryAction.href}
          data-testid="obsidian-nav-primary-action"
          className="inline-flex items-center gap-2 rounded-lg px-3 py-2 text-[11px] font-bold uppercase tracking-[0.16em]"
          style={{ background: 'var(--primary-container)', color: 'var(--on-primary-container)' }}
        >
          {primaryAction.label}
          <FaArrowRight size={11} />
        </Link>
      </div>
    </header>
  );
}
