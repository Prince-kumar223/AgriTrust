import { ArrowRight, ShieldCheck, Sprout, WalletCards } from 'lucide-react';
import { Link } from 'react-router-dom';

import { Button } from '../components/Button';
import { Card } from '../components/Card';

export function LandingPage() {
  return (
    <main className="min-h-screen bg-[#F7F4EC] text-[#172018]">
      <section className="mx-auto grid min-h-[92vh] max-w-7xl items-center gap-10 px-4 py-10 lg:grid-cols-[1.1fr_0.9fr]">
        <div>
          <p className="mb-3 text-sm font-bold uppercase tracking-[0.18em] text-[#D4A017]">
            Soroban escrow for agriculture
          </p>
          <h1 className="max-w-3xl text-4xl font-bold leading-tight text-[#1B4332] sm:text-5xl lg:text-6xl">
            Trade crops with payment certainty from offer to delivery.
          </h1>
          <p className="mt-5 max-w-2xl text-lg leading-8 text-[#4A5649]">
            AgriTrust gives farmers and buyers a shared workflow for listings, offers, escrowed
            payments, delivery confirmation, and feedback.
          </p>
          <div className="mt-8 flex flex-wrap gap-3">
            <Link to="/wallet">
              <Button icon={<WalletCards size={18} />}>Connect Wallet</Button>
            </Link>
            <Link to="/buyer">
              <Button variant="secondary" icon={<ArrowRight size={18} />}>
                Browse Market
              </Button>
            </Link>
          </div>
        </div>
        <Card className="grid gap-4">
          {[
            [
              'Problem',
              'Crop deals often rely on informal trust, slow follow-up, and unclear payment status.',
            ],
            ['Solution', 'Escrow keeps funds locked until the buyer confirms delivery.'],
            ['Outcome', 'Farmers see committed buyers. Buyers get visible trade milestones.'],
          ].map(([title, copy]) => (
            <div key={title} className="rounded-md bg-[#F7F4EC] p-4">
              <h2 className="font-semibold text-[#1B4332]">{title}</h2>
              <p className="mt-1 text-sm leading-6 text-[#5F695D]">{copy}</p>
            </div>
          ))}
        </Card>
      </section>
      <section className="mx-auto grid max-w-7xl gap-4 px-4 pb-12 md:grid-cols-3">
        {[
          {
            icon: Sprout,
            title: 'List produce',
            copy: 'Farmers publish quantity, unit, pricing, and location.',
          },
          {
            icon: ShieldCheck,
            title: 'Escrow payment',
            copy: 'Accepted offers move into Soroban-backed trade records.',
          },
          {
            icon: WalletCards,
            title: 'Wallet first',
            copy: 'Freighter signs login and trade actions without passwords.',
          },
        ].map(({ icon: Icon, title, copy }) => (
          <Card key={title}>
            <Icon className="text-[#D4A017]" />
            <h2 className="mt-4 font-semibold text-[#1B4332]">{title}</h2>
            <p className="mt-2 text-sm leading-6 text-[#5F695D]">{copy}</p>
          </Card>
        ))}
      </section>
    </main>
  );
}
