import HomeLandingClient from '@/components/ui/HomeLandingClient';
import { loadInitialHomeFeed } from '@/lib/server/home-feed';

export const dynamic = 'force-dynamic';

export default async function AppHomePage() {
  const initialFeed = await loadInitialHomeFeed();
  return <HomeLandingClient initialFeed={initialFeed} />;
}
