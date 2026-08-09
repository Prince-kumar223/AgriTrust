import { CheckCircle2, Coins, PackageCheck } from 'lucide-react';
import { useState } from 'react';
import { useParams } from 'react-router-dom';
import { toast } from 'sonner';

import { Button } from '../components/Button';
import { Card } from '../components/Card';
import { FeedbackModal } from '../components/FeedbackModal';
import { Skeleton } from '../components/Skeleton';
import { StateSteps } from '../components/StateSteps';
import { useRecordTradeTransition, useTrade } from '../hooks/useAgriTrustData';
import { signEscrowTransaction } from '../lib/freighter';

const transitionByStatus = {
  pending: { action: 'record_deposit', label: 'Deposit Payment', icon: Coins },
  funded: { action: 'record_delivery', label: 'Confirm Delivery', icon: PackageCheck },
  delivered: { action: 'record_confirmation', label: 'Confirm Receipt', icon: CheckCircle2 },
};

export function TradeDetailPage() {
  const { tradeId } = useParams();
  const trade = useTrade(tradeId);
  const transition = useRecordTradeTransition(tradeId);
  const [showFeedback, setShowFeedback] = useState(false);

  async function performAction() {
    if (!trade.data) return;
    const next = transitionByStatus[trade.data.status as keyof typeof transitionByStatus];
    if (!next) return;
    try {
      await signEscrowTransaction(
        'PLACEHOLDER_CONTRACT_CALL_XDR',
        'Test SDF Network ; September 2015',
      );
      transition.mutate(
        { action: next.action, tx_hash: `tx-${Date.now()}` },
        {
          onSuccess: (updatedTrade) => {
            if (updatedTrade.status === 'completed') setShowFeedback(true);
          },
        },
      );
    } catch {
      toast.error('Transaction was not signed. Please check Freighter and retry.');
    }
  }

  if (trade.isLoading) {
    return (
      <main className="mx-auto max-w-5xl px-4 py-8">
        <Skeleton className="h-72 w-full" />
      </main>
    );
  }

  if (trade.isError || !trade.data) {
    return (
      <main className="mx-auto max-w-5xl px-4 py-8">
        <Card>
          <p className="text-red-700">Trade details could not load.</p>
          <Button className="mt-3" onClick={() => trade.refetch()}>
            Retry
          </Button>
        </Card>
      </main>
    );
  }

  const action = transitionByStatus[trade.data.status as keyof typeof transitionByStatus];
  const ActionIcon = action?.icon;

  return (
    <main className="mx-auto max-w-5xl px-4 py-8">
      <h1 className="text-3xl font-bold text-[#1B4332]">Trade {trade.data.on_chain_trade_id}</h1>
      <Card className="mt-6">
        <StateSteps status={trade.data.status} />
        <dl className="mt-6 grid gap-4 sm:grid-cols-2">
          <div>
            <dt className="text-sm text-[#5F695D]">Contract</dt>
            <dd className="break-all font-medium">{trade.data.contract_address}</dd>
          </div>
          <div>
            <dt className="text-sm text-[#5F695D]">Current status</dt>
            <dd className="font-medium capitalize">{trade.data.status}</dd>
          </div>
        </dl>
        <div className="mt-6 flex flex-wrap gap-3">
          {action ? (
            <Button
              icon={ActionIcon ? <ActionIcon size={18} /> : undefined}
              isLoading={transition.isPending}
              onClick={performAction}
            >
              {action.label}
            </Button>
          ) : null}
          {trade.data.status === 'completed' ? (
            <Button variant="secondary" onClick={() => setShowFeedback(true)}>
              Leave Feedback
            </Button>
          ) : null}
        </div>
      </Card>
      {showFeedback ? (
        <FeedbackModal tradeId={trade.data.id} onClose={() => setShowFeedback(false)} />
      ) : null}
    </main>
  );
}
