import type { Metadata } from 'next';
import { Inter, Manrope, Roboto_Mono } from 'next/font/google';
import './globals.css';

const inter = Inter({
  subsets: ['latin'],
  variable: '--font-body',
  weight: ['400', '500', '600', '700'],
});

const manrope = Manrope({
  subsets: ['latin'],
  variable: '--font-headline',
  weight: ['600', '700', '800'],
});

const robotoMono = Roboto_Mono({
  subsets: ['latin'],
  variable: '--font-mono',
  weight: ['400', '500', '700'],
});

export const metadata: Metadata = {
  title: 'MIA | Agentic Investigation for Four.Meme',
  description:
    'Evidence-first agentic investigation for Four.Meme. Discover launches, inspect runs, review holder structure, and trigger deep research on BNB Chain.',
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body className={`${inter.variable} ${manrope.variable} ${robotoMono.variable} app-shell antialiased`}>
        {children}
      </body>
    </html>
  );
}
