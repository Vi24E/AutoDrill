import type { Metadata } from 'next';
import '@fontsource/noto-sans-jp/400.css';
import 'mathlive/fonts.css';
import './globals.css';

export const metadata: Metadata = {
  title: 'AutoDrill | 計算ドリル',
  description: '白黒の計算ドリルをつくって、解いて、採点します。',
};

export default function RootLayout({ children }: Readonly<{ children: React.ReactNode }>) {
  return <html lang="ja"><body>{children}</body></html>;
}
