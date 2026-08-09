import { Send } from 'lucide-react';
import type { FormEvent } from 'react';

import { Button } from '../components/Button';
import { Card } from '../components/Card';
import { Skeleton } from '../components/Skeleton';
import {
  useAcceptOffer,
  useCreateListing,
  useListings,
  useOffers,
  useTrades,
} from '../hooks/useAgriTrustData';

export function FarmerDashboardPage() {
  const listings = useListings();
  const offers = useOffers();
  const trades = useTrades();
  const createListing = useCreateListing();
  const acceptOffer = useAcceptOffer();

  function submitListing(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    createListing.mutate(Object.fromEntries(new FormData(event.currentTarget)));
    event.currentTarget.reset();
  }

  return (
    <main className="mx-auto grid max-w-7xl gap-6 px-4 py-8 lg:grid-cols-[0.9fr_1.1fr]">
      <section>
        <h1 className="text-3xl font-bold text-[#1B4332]">Farmer dashboard</h1>
        <Card className="mt-6">
          <h2 className="font-semibold text-[#1B4332]">Create listing</h2>
          <form className="mt-4 grid gap-3" onSubmit={submitListing}>
            {['crop_type', 'quantity', 'unit', 'price_per_unit'].map((name) => (
              <input
                key={name}
                required
                name={name}
                className="rounded-md border border-[#DAD2BE] px-3 py-2 outline-none focus:border-[#1B4332]"
                placeholder={name.replaceAll('_', ' ')}
              />
            ))}
            <textarea
              name="description"
              className="min-h-24 rounded-md border border-[#DAD2BE] px-3 py-2 outline-none focus:border-[#1B4332]"
              placeholder="Description"
            />
            <Button icon={<Send size={16} />} isLoading={createListing.isPending}>
              Publish Listing
            </Button>
          </form>
        </Card>
      </section>

      <section className="grid gap-6">
        <Card>
          <h2 className="font-semibold text-[#1B4332]">My listings</h2>
          <ListState
            isLoading={listings.isLoading}
            isError={listings.isError}
            onRetry={listings.refetch}
          />
          <div className="mt-4 grid gap-3">
            {listings.data?.map((listing) => (
              <div key={listing.id} className="rounded-md bg-[#F7F4EC] p-3">
                <div className="flex justify-between gap-3">
                  <strong>{listing.crop_type}</strong>
                  <span className="text-sm capitalize text-[#1B4332]">{listing.status}</span>
                </div>
                <p className="text-sm text-[#5F695D]">
                  {listing.quantity} {listing.unit} at {listing.price_per_unit}
                </p>
              </div>
            ))}
          </div>
        </Card>

        <Card>
          <h2 className="font-semibold text-[#1B4332]">Incoming offers</h2>
          <ListState
            isLoading={offers.isLoading}
            isError={offers.isError}
            onRetry={offers.refetch}
          />
          <div className="mt-4 grid gap-3">
            {offers.data?.map((offer) => (
              <div
                key={offer.id}
                className="flex flex-col gap-3 rounded-md bg-[#F7F4EC] p-3 sm:flex-row sm:items-center sm:justify-between"
              >
                <div>
                  <strong>{offer.offered_price}</strong>
                  <p className="text-sm text-[#5F695D]">{offer.message || 'No message'}</p>
                </div>
                {offer.status === 'pending' ? (
                  <Button
                    isLoading={acceptOffer.isPending}
                    onClick={() => acceptOffer.mutate(offer.id)}
                  >
                    Accept
                  </Button>
                ) : (
                  <span className="text-sm capitalize text-[#1B4332]">{offer.status}</span>
                )}
              </div>
            ))}
          </div>
        </Card>

        <Card>
          <h2 className="font-semibold text-[#1B4332]">Active trades</h2>
          <ListState
            isLoading={trades.isLoading}
            isError={trades.isError}
            onRetry={trades.refetch}
          />
          <div className="mt-4 grid gap-3">
            {trades.data?.map((trade) => (
              <a
                key={trade.id}
                href={`/trades/${trade.id}`}
                className="rounded-md bg-[#F7F4EC] p-3 text-sm hover:bg-[#EFE7D3]"
              >
                {trade.on_chain_trade_id} · <span className="capitalize">{trade.status}</span>
              </a>
            ))}
          </div>
        </Card>
      </section>
    </main>
  );
}

function ListState({
  isLoading,
  isError,
  onRetry,
}: {
  isLoading: boolean;
  isError: boolean;
  onRetry: () => void;
}) {
  if (isLoading) return <Skeleton className="mt-4 h-24 w-full" />;
  if (isError) {
    return (
      <div className="mt-4 rounded-md bg-red-50 p-3 text-sm text-red-700">
        Could not load this section. <button onClick={onRetry}>Retry</button>
      </div>
    );
  }
  return null;
}
