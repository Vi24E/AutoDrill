import type { Metadata } from 'next';
import '@fontsource/noto-sans-jp/400.css';
import 'mathlive/fonts.css';
import './globals.css';

const alphaPublicPreview = process.env.NEXT_PUBLIC_DEPLOY_CHANNEL === 'alpha';

export const metadata: Metadata = {
  title: 'AutoDrill | 計算ドリル',
  description: '白黒の計算ドリルをつくって、解いて、採点します。',
  ...(alphaPublicPreview ? { robots: { index: false, follow: false } } : {}),
};

export default function RootLayout({ children }: Readonly<{ children: React.ReactNode }>) {
  return (
    <html lang="ja">
      {/* Accessibility/reading extensions can add attributes directly to body
          before React hydrates. Those attributes are outside AutoDrill's state
          and should not turn an otherwise valid page into a hydration warning. */}
      <body suppressHydrationWarning>{children}</body>
    </html>
  );
}
