/**
 * r[impl jetstream.rpc.ts.protocol]
 */
import type { Framer } from "./frame.js";

export interface Protocol<TReq extends Framer, TRes extends Framer> {
  readonly VERSION: string;
  readonly NAME: string;
}
