/**
 * r[impl jetstream.react.use-jetstream]
 * r[impl jetstream.react.use-jetstream.type-safety]
 * r[impl jetstream.react.use-jetstream.identity]
 * r[impl jetstream.version.framer.client-handshake]
 */
import { useContext, useEffect, useRef, useState } from "react";
import { Mux } from "@sevki/jetstream-rpc";
import type { Framer, FramerDecode } from "@sevki/jetstream-rpc";
import { negotiateVersion } from "@sevki/jetstream-rpc";
import { JetStreamContext } from "./provider.js";
import { WebTransportTransport } from "@sevki/jetstream-http";

type ClientConstructor<TReq extends Framer, TRes extends Framer, C> = new (
  mux: Mux<TReq, TRes>,
) => C;

/**
 * Returns a memoized client instance for calling upstream services.
 * The generated client constructor accepts a Mux, and the hook
 * creates a WebTransport bidi stream, performs Tversion/Rversion
 * negotiation, then wraps the stream in a Transport + Mux and
 * supplies it to the client. The negotiated protocol version is
 * stored in the JetStreamContext.
 */
export function useJetStream<TReq extends Framer, TRes extends Framer, C>(
  ClientClass: ClientConstructor<TReq, TRes, C>,
  responseDecode: FramerDecode<TRes>,
  protocolVersion: string,
): C | null {
  const ctx = useContext(JetStreamContext);
  if (!ctx)
    throw new Error("useJetStream must be used within JetStreamProvider");

  const [client, setClient] = useState<C | null>(null);
  const muxRef = useRef<Mux<TReq, TRes> | null>(null);

  useEffect(() => {
    if (!ctx.session) return;
    let cancelled = false;

    async function setup() {
      const stream = await ctx!.session!.createBidirectionalStream();
      if (cancelled) return;

      // Negotiate version before handing stream to Mux
      const negotiated = await negotiateVersion(
        stream.readable,
        stream.writable,
        protocolVersion,
      );
      if (cancelled) return;

      ctx!.setProtocolVersion(negotiated.version);

      const transport = new WebTransportTransport<TReq, TRes>(
        stream,
        responseDecode,
      );
      const mux = new Mux(transport);
      muxRef.current = mux;
      await mux.start();

      if (cancelled) {
        await mux.close();
        return;
      }

      setClient(new ClientClass(mux));
    }

    setup().catch(() => {
      if (!cancelled) setClient(null);
    });

    return () => {
      cancelled = true;
      muxRef.current?.close();
      muxRef.current = null;
    };
  }, [ctx.session, ClientClass, responseDecode, protocolVersion]);

  return client;
}
