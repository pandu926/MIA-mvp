import { redirect } from 'next/navigation';

interface Props {
  params: Promise<{ address: string }>;
}

export default async function TokenAnalysisPage({ params }: Props) {
  const { address } = await params;
  redirect(`/mia?q=${encodeURIComponent(address)}`);
}
