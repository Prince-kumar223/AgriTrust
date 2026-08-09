import type { TradeStatus } from '../lib/types';

const steps: TradeStatus[] = ['pending', 'funded', 'delivered', 'completed'];

export function StateSteps({ status }: { status: TradeStatus }) {
  const activeIndex = steps.indexOf(status);
  return (
    <div className="grid gap-3 sm:grid-cols-4">
      {steps.map((step, index) => {
        const isDone = status === 'disputed' ? index <= activeIndex : index <= activeIndex;
        return (
          <div key={step} className="flex items-center gap-2">
            <div
              className={[
                'flex h-8 w-8 shrink-0 items-center justify-center rounded-full text-sm font-bold',
                isDone ? 'bg-[#1B4332] text-white' : 'bg-[#E8E0CD] text-[#6B705C]',
              ].join(' ')}
            >
              {index + 1}
            </div>
            <span className="capitalize text-sm font-medium text-[#263326]">{step}</span>
          </div>
        );
      })}
      {status === 'disputed' ? (
        <p className="sm:col-span-4 rounded-md bg-red-50 px-3 py-2 text-sm text-red-700">
          This trade is disputed. Funds remain locked until the parties resolve it.
        </p>
      ) : null}
    </div>
  );
}
