import { RefreshCcw } from 'lucide-react';
import { useEffect, useState } from 'react';

import { useFlushTradeSyncQueue } from '../hooks/useAgriTrustData';
import { pendingTradeSyncCount } from '../lib/syncQueue';
import { Button } from './Button';

export function TradeSyncButton() {
  const flushQueue = useFlushTradeSyncQueue();
  const [queuedSyncs, setQueuedSyncs] = useState(pendingTradeSyncCount());

  useEffect(() => {
    const updateCount = () => setQueuedSyncs(pendingTradeSyncCount());
    window.addEventListener('agritrust-sync-queue', updateCount);
    return () => window.removeEventListener('agritrust-sync-queue', updateCount);
  }, []);

  if (queuedSyncs === 0) return null;

  return (
    <Button
      variant="secondary"
      icon={<RefreshCcw size={18} />}
      isLoading={flushQueue.isPending}
      onClick={() => flushQueue.mutate()}
    >
      Sync Pending Updates
    </Button>
  );
}
