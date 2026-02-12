/**
 * r[impl jetstream.rpc.ts.transport]
 */
import type { Frame, Framer } from './frame.js';

export interface Transport<TReq extends Framer, TRes extends Framer> {
  send(frame: Frame<TReq>): Promise<void>;
  receive(): AsyncIterable<Frame<TRes>>;
  close(): Promise<void>;
}
