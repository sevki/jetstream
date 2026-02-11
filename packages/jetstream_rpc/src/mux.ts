/**
 * r[impl jetstream.rpc.ts.mux]
 */
import type { Frame, Framer } from './frame.js';
import type { Transport } from './transport.js';
import { TagPool } from './tag-pool.js';

export class Mux<TReq extends Framer, TRes extends Framer> {
  private tagPool: TagPool;
  private pending: Map<number, { resolve: (frame: Frame<TRes>) => void; reject: (err: Error) => void }> = new Map();
  private transport: Transport<TReq, TRes>;
  private running = false;

  constructor(transport: Transport<TReq, TRes>, maxConcurrent: number = 256) {
    this.transport = transport;
    this.tagPool = new TagPool(maxConcurrent);
  }

  async start(): Promise<void> {
    this.running = true;
    (async () => {
      for await (const frame of this.transport.receive()) {
        const pending = this.pending.get(frame.tag);
        if (pending) {
          this.pending.delete(frame.tag);
          this.tagPool.release(frame.tag);
          pending.resolve(frame);
        }
      }
    })();
  }

  async rpc(msg: TReq): Promise<Frame<TRes>> {
    const tag = this.tagPool.acquire();
    if (tag === null) throw new Error('no tags available');

    return new Promise((resolve, reject) => {
      this.pending.set(tag, { resolve, reject });
      this.transport.send({ tag, msg }).catch(reject);
    });
  }

  async close(): Promise<void> {
    this.running = false;
    await this.transport.close();
  }
}
