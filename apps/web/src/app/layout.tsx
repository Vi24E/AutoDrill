import type { Metadata } from 'next';
import '@fontsource/noto-sans-jp/400.css';
import 'mathlive/fonts.css';
import './globals.css';

export const metadata: Metadata = {
  title: 'AutoDrill | 計算ドリル',
  description: '白黒の計算ドリルをつくって、解いて、採点します。',
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
