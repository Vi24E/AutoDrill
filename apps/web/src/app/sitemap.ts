import type { MetadataRoute } from 'next';

import { IMPLEMENTED_THEMES } from '@/domain/curriculum';

export const dynamic = 'force-static';

function publicUrl(pathname: string): string {
  const siteOrigin = (process.env.NEXT_PUBLIC_SITE_URL ?? 'http://localhost:3000').replace(/\/$/, '');
  const basePath = process.env.NEXT_PUBLIC_BASE_PATH ?? '';
  const suffix = pathname === '/' ? '/' : pathname;
  return `${siteOrigin}${basePath}${suffix}`;
}

export default function sitemap(): MetadataRoute.Sitemap {
  return [
    { url: publicUrl('/') },
    ...IMPLEMENTED_THEMES.map((theme) => ({
      url: publicUrl(theme.route.pathname),
    })),
  ];
}
