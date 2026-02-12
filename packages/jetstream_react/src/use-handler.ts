/**
 * r[impl jetstream.react.use-handler]
 * r[impl jetstream.react.use-handler.state]
 * r[impl jetstream.react.use-handler.type-safety]
 * r[impl jetstream.react.use-handler.lifecycle]
 */
import { useContext, useEffect, useRef, useState } from "react";
import type { FramerCodec, ServerCodec } from "@sevki/jetstream-rpc";
import { ServerCodec as ServerCodecClass } from "@sevki/jetstream-rpc";
import type { Context } from "@sevki/jetstream-rpc";
import { JetStreamContext } from "./provider.js";

export interface HandlerEvent {
  method: string;
  args: unknown[];
  result: unknown;
  timestamp: number;
}

export interface UseHandlerResult {
  events: HandlerEvent[];
  error: Error | undefined;
}

interface HandlerRegistration<TReq, TRes> {
  protocolName: string;
  requestCodec: FramerCodec<TReq>;
  responseCodec: FramerCodec<TRes>;
  dispatch: (
    handler: Record<
      string,
      (ctx: Context, ...args: unknown[]) => Promise<unknown>
    >,
    ctx: Context,
    frame: { tag: number; msg: TReq },
  ) => Promise<{ tag: number; msg: TRes }>;
}

/**
 * Registers a handler for incoming upstream RPCs and provides reactive state.
 * When upstream opens a bidirectional stream and sends request frames,
 * the handler's methods are invoked. Returns { events, error } where events
 * is a reactive array that updates on each incoming call.
 */
export function useHandler<
  THandler extends Record<string, (...args: any[]) => Promise<any>>,
>(
  registration: HandlerRegistration<unknown, unknown>,
  impl: THandler,
): UseHandlerResult {
  const ctx = useContext(JetStreamContext);
  if (!ctx) throw new Error("useHandler must be used within JetStreamProvider");

  const [events, setEvents] = useState<HandlerEvent[]>([]);
  const [error, setError] = useState<Error | undefined>(undefined);
  const implRef = useRef(impl);
  implRef.current = impl;

  useEffect(() => {
    const key = registration.protocolName;

    const createCodec = () =>
      new ServerCodecClass(
        registration.requestCodec,
        registration.responseCodec,
      );

    const dispatch = async (frame: { tag: number; msg: unknown }) => {
      try {
        const rpcCtx: Context = {};
        const result = await registration.dispatch(
          implRef.current as Record<
            string,
            (ctx: Context, ...args: unknown[]) => Promise<unknown>
          >,
          rpcCtx,
          frame as { tag: number; msg: any },
        );

        // Record the event for reactive state
        const msgObj = frame.msg as { type?: string };
        setEvents((prev) => [
          ...prev,
          {
            method: msgObj.type ?? "unknown",
            args: [],
            result,
            timestamp: Date.now(),
          },
        ]);

        return result;
      } catch (err) {
        setError(err instanceof Error ? err : new Error(String(err)));
        throw err;
      }
    };

    ctx.handlers.set(key, {
      createCodec: createCodec as () => ServerCodec<unknown, unknown>,
      dispatch: dispatch as any,
    });

    return () => {
      ctx.handlers.delete(key);
    };
  }, [ctx, registration]);

  return { events, error };
}
