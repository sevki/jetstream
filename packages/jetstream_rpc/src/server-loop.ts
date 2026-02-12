/**
 * r[impl jetstream.rpc.ts.server-loop]
 */
import type { ServerCodec } from './server-codec.js';

export type DispatchFn<TReq, TRes> = (
  frame: { tag: number; msg: TReq },
) => Promise<{ tag: number; msg: TRes }>;

/**
 * Read request frames from a ReadableStream via ServerCodec, dispatch each
 * to the handler's dispatch function concurrently, and write response frames
 * to the WritableStream via a write queue.
 */
export async function serverLoop<TReq, TRes>(
  codec: ServerCodec<TReq, TRes>,
  dispatch: DispatchFn<TReq, TRes>,
  readable: ReadableStream<Uint8Array>,
  writable: WritableStream<Uint8Array>,
): Promise<void> {
  const writer = writable.getWriter();
  const pending: Promise<void>[] = [];

  try {
    for await (const frame of codec.decodeRequests(readable)) {
      // Dispatch concurrently — each incoming frame is handed off immediately
      const p = dispatch(frame)
        .then((response) => {
          const bytes = codec.encodeResponse(response);
          return writer.write(bytes);
        })
        .catch(() => {
          // dispatch errors are handled inside dispatch (wrapped as Error frames)
        });
      pending.push(p);
    }
    // Wait for all in-flight dispatches to complete
    await Promise.all(pending);
  } finally {
    writer.releaseLock();
  }
}

/**
 * r[impl jetstream.rpc.ts.server-loop.transport]
 *
 * Accept incoming bidirectional streams and run a server loop on each.
 * On WebTransport, this listens on session.incomingBidirectionalStreams.
 */
export async function acceptAndServe<TReq, TRes>(
  incomingStreams: ReadableStream<{
    readable: ReadableStream<Uint8Array>;
    writable: WritableStream<Uint8Array>;
  }>,
  createCodec: () => ServerCodec<TReq, TRes>,
  dispatch: DispatchFn<TReq, TRes>,
): Promise<void> {
  const reader = incomingStreams.getReader();
  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      // Each accepted stream gets its own codec instance and server loop
      const codec = createCodec();
      serverLoop(codec, dispatch, value.readable, value.writable).catch(
        () => {},
      );
    }
  } finally {
    reader.releaseLock();
  }
}
