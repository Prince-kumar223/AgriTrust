import { AlertCircle, CheckCircle2, Lock, WalletCards } from 'lucide-react';

import { Button } from '../components/Button';
import { Card } from '../components/Card';
import { Skeleton } from '../components/Skeleton';
import { useWallet } from '../hooks/useWallet';

export function WalletPage() {
  const { state, connect } = useWallet();

  return (
    <main className="mx-auto max-w-4xl px-4 py-8">
      <h1 className="text-3xl font-bold text-[#1B4332]">Wallet connection</h1>
      <p className="mt-2 text-[#5F695D]">
        Connect Freighter to sign in and approve escrow actions.
      </p>

      <Card className="mt-6">
        {state.isLoading ? (
          <div className="space-y-3">
            <Skeleton className="h-8 w-48" />
            <Skeleton className="h-20 w-full" />
          </div>
        ) : state.data?.status === 'connected' ? (
          <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
            <div className="flex items-center gap-3">
              <CheckCircle2 className="text-[#1B4332]" />
              <div>
                <h2 className="font-semibold">Connected</h2>
                <p className="break-all text-sm text-[#5F695D]">{state.data.address}</p>
              </div>
            </div>
            <Button variant="secondary" onClick={() => state.refetch()}>
              Refresh
            </Button>
          </div>
        ) : (
          <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
            <div className="flex items-center gap-3">
              {state.data?.status === 'locked' ? (
                <Lock className="text-[#D4A017]" />
              ) : (
                <AlertCircle className="text-red-600" />
              )}
              <div>
                <h2 className="font-semibold">
                  {state.data?.status === 'locked'
                    ? 'Freighter needs approval'
                    : 'Freighter not installed'}
                </h2>
                <p className="text-sm text-[#5F695D]">
                  {state.data?.message ?? 'Wallet status unavailable.'}
                </p>
              </div>
            </div>
            <Button
              icon={<WalletCards size={18} />}
              isLoading={connect.isPending}
              onClick={() => connect.mutate()}
            >
              Connect Wallet
            </Button>
          </div>
        )}
      </Card>
    </main>
  );
}
