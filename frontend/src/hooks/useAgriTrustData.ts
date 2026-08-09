import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';

import { api } from '../lib/api';
import type { AnalyticsSummary, CropListing, Offer, Trade } from '../lib/types';

export function useListings() {
  return useQuery({
    queryKey: ['listings'],
    queryFn: async () => (await api.get<CropListing[]>('/listings/')).data,
  });
}

export function useOffers() {
  return useQuery({
    queryKey: ['offers'],
    queryFn: async () => (await api.get<Offer[]>('/offers/')).data,
  });
}

export function useTrades() {
  return useQuery({
    queryKey: ['trades'],
    queryFn: async () => (await api.get<Trade[]>('/trades/')).data,
  });
}

export function useTrade(id?: string) {
  return useQuery({
    queryKey: ['trade', id],
    enabled: Boolean(id),
    queryFn: async () => (await api.get<Trade>(`/trades/${id}/`)).data,
  });
}

export function useAnalytics() {
  return useQuery({
    queryKey: ['analytics-summary'],
    queryFn: async () => (await api.get<AnalyticsSummary>('/analytics/summary/')).data,
  });
}

export function useCreateListing() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (payload: Record<string, FormDataEntryValue>) =>
      (await api.post<CropListing>('/listings/', payload)).data,
    onSuccess: () => {
      toast.success('Listing created');
      void queryClient.invalidateQueries({ queryKey: ['listings'] });
    },
    onError: showMutationError,
  });
}

export function useCreateOffer() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (payload: { listing: number; offered_price: string; message: string }) =>
      (await api.post<Offer>('/offers/', payload)).data,
    onSuccess: () => {
      toast.success('Offer sent');
      void queryClient.invalidateQueries({ queryKey: ['offers'] });
      void queryClient.invalidateQueries({ queryKey: ['listings'] });
    },
    onError: showMutationError,
  });
}

export function useAcceptOffer() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (offerId: number) =>
      (await api.post<Offer>(`/offers/${offerId}/accept/`)).data,
    onSuccess: () => {
      toast.success('Offer accepted');
      void queryClient.invalidateQueries({ queryKey: ['offers'] });
      void queryClient.invalidateQueries({ queryKey: ['listings'] });
    },
    onError: showMutationError,
  });
}

export function useRecordTradeTransition(tradeId?: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({ action, tx_hash }: { action: string; tx_hash: string }) =>
      (await api.post<Trade>(`/trades/${tradeId}/${action}/`, { tx_hash })).data,
    onSuccess: () => {
      toast.success('Trade updated');
      void queryClient.invalidateQueries({ queryKey: ['trade', tradeId] });
      void queryClient.invalidateQueries({ queryKey: ['trades'] });
    },
    onError: showMutationError,
  });
}

function showMutationError(error: unknown) {
  toast.error(error instanceof Error ? error.message : 'Action failed. Please retry.');
}
