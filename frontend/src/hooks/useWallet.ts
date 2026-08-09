import { useMutation, useQuery } from '@tanstack/react-query';
import { toast } from 'sonner';

import { connectFreighter, getWalletState } from '../lib/freighter';

export function useWallet() {
  const state = useQuery({
    queryKey: ['wallet-state'],
    queryFn: getWalletState,
  });

  const connect = useMutation({
    mutationFn: connectFreighter,
    onSuccess: () => {
      toast.success('Wallet connected');
      void state.refetch();
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : 'Wallet connection failed.');
    },
  });

  return { state, connect };
}
