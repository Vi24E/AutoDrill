import type { MetadataRoute } from 'next';

import { IMPLEMENTED_THEMES } from '@/domain/curriculum';

const SITE_ORIGIN = process.env.NEXT_PUBLIC_SITE_URL ?? 'http://localhost:3000';

export default function sitemap(): MetadataRoute.Sitemap {
  return [
    { url: new URL('/', SITE_ORIGIN).toString() },
    ...IMPLEMENTED_THEMES.map((theme) => ({
      url: new URL(theme.route.pathname, SITE_ORIGIN).toString(),
    })),
  ];
}
