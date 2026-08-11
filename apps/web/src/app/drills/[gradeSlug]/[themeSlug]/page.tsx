import type { Metadata } from 'next';
import { notFound } from 'next/navigation';

import { AutoDrillApp } from '@/components/AutoDrillApp';
import {
  IMPLEMENTED_THEMES,
  createWebDrillSettings,
  findImplementedThemeByRoute,
} from '@/domain/curriculum';

type UnitPageParams = {
  gradeSlug: string;
  themeSlug: string;
};

type UnitPageProps = {
  params: Promise<UnitPageParams>;
};

export const dynamicParams = false;

export function generateStaticParams(): UnitPageParams[] {
  return IMPLEMENTED_THEMES.map((theme) => ({
    gradeSlug: theme.route.gradeSlug,
    themeSlug: theme.route.themeSlug,
  }));
}

export async function generateMetadata({ params }: UnitPageProps): Promise<Metadata> {
  const { gradeSlug, themeSlug } = await params;
  const theme = findImplementedThemeByRoute(gradeSlug, themeSlug);
  if (!theme) notFound();
  return {
    title: theme.search.title,
    description: theme.search.description,
  };
}

export default async function UnitPage({ params }: UnitPageProps) {
  const { gradeSlug, themeSlug } = await params;
  const theme = findImplementedThemeByRoute(gradeSlug, themeSlug);
  if (!theme) notFound();
  return <AutoDrillApp initialWebSettings={createWebDrillSettings(theme)} />;
}
