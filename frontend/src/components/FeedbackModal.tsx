import { Star } from 'lucide-react';
import { useState } from 'react';
import { toast } from 'sonner';

import { api } from '../lib/api';
import { Button } from './Button';
import { Card } from './Card';

export function FeedbackModal({ tradeId, onClose }: { tradeId: number; onClose: () => void }) {
  const [rating, setRating] = useState(5);
  const [comment, setComment] = useState('');
  const [isSaving, setIsSaving] = useState(false);

  async function submit() {
    setIsSaving(true);
    try {
      await api.post('/feedback/', { trade: tradeId, rating, comment });
      toast.success('Feedback submitted');
      onClose();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Could not submit feedback.');
    } finally {
      setIsSaving(false);
    }
  }

  return (
    <div className="fixed inset-0 z-40 grid place-items-center bg-black/40 p-4">
      <Card className="w-full max-w-md">
        <h2 className="text-xl font-semibold text-[#1B4332]">How did this trade go?</h2>
        <div className="mt-4 flex gap-2">
          {[1, 2, 3, 4, 5].map((value) => (
            <button
              key={value}
              className={value <= rating ? 'text-[#D4A017]' : 'text-[#C9C0AA]'}
              onClick={() => setRating(value)}
              type="button"
            >
              <Star fill="currentColor" size={28} />
            </button>
          ))}
        </div>
        <textarea
          className="mt-4 min-h-28 w-full rounded-md border border-[#DAD2BE] p-3 outline-none focus:border-[#1B4332]"
          value={comment}
          onChange={(event) => setComment(event.target.value)}
          placeholder="Share a short note"
        />
        <div className="mt-4 flex justify-end gap-2">
          <Button variant="ghost" onClick={onClose}>
            Later
          </Button>
          <Button isLoading={isSaving} onClick={submit}>
            Submit
          </Button>
        </div>
      </Card>
    </div>
  );
}
