/**
 * r[impl jetstream.react.provider]
 * r[impl jetstream.react.provider.connection-state]
 */
import { createContext, useContext, useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import type { FramerCodec, ServerCodec } from "@sevki/jetstream-rpc";
import type { DispatchFn } from "@sevki/jetstream-rpc";
import { serverLoop, acceptVersion } from "@sevki/jetstream-rpc";

export type ConnectionState =
  | "connecting"
  | "connected"
  | "disconnected"
  | "error";

export interface JetStreamContextValue {
  session: WebTransport | null;
  state: ConnectionState;
  protocolVersion: string | null;
  setProtocolVersion: (version: string) => void;
  handlers: Map<
    string,
    {
      createCodec: () => ServerCodec<unknown, unknown>;
      dispatch: DispatchFn<unknown, unknown>;
    }
  >;
}

export const JetStreamContext = createContext<JetStreamContextValue | null>(
  null,
);

export interface JetStreamProviderProps {
  url: string;
  /** Optional async function that returns a certificate string (e.g. base64 DER).
   *  When provided, the certificate is URL-encoded and appended as `?cert=<encoded>`
   *  to the WebTransport URL before connecting. */
  getCertificate?: () => Promise<string>;
  maxConcurrentRequests?: number;
  children: ReactNode;
}

export function JetStreamProvider({
  url,
  getCertificate,
  children,
}: JetStreamProviderProps) {
  const [session, setSession] = useState<WebTransport | null>(null);
  const [state, setState] = useState<ConnectionState>("connecting");
  const [protocolVersion, setProtocolVersion] = useState<string | null>(null);
  const handlersRef = useRef<JetStreamContextValue["handlers"]>(new Map());

  useEffect(() => {
    let transport: WebTransport | null = null;
    let cancelled = false;

    async function connect() {
      try {
        let connectUrl = url;
        if (getCertificate) {
          const cert = await getCertificate();
          if (cancelled) return;
          const encoded = encodeURIComponent(cert);
          const separator = url.includes("?") ? "&" : "?";
          connectUrl = `${url}${separator}cert=${encoded}`;
        }

        transport = new WebTransport(connectUrl);
        await transport.ready;
        if (cancelled) {
          transport.close();
          return;
        }
        setSession(transport);
        setState("connected");

        // r[impl jetstream.react.webtransport.accept]
        // Accept loop for incoming bidi streams from upstream
        acceptIncoming(transport, handlersRef.current).catch(() => {});

        await transport.closed;
        if (!cancelled) {
          setState("disconnected");
          setSession(null);
        }
      } catch {
        if (!cancelled) {
          setState("error");
          setSession(null);
        }
      }
    }

    connect();

    return () => {
      cancelled = true;
      transport?.close();
    };
  }, [url, getCertificate]);

  const contextValue: JetStreamContextValue = {
    session,
    state,
    protocolVersion,
    setProtocolVersion,
    handlers: handlersRef.current,
  };

  return (
    <JetStreamContext.Provider value={contextValue}>
      {children}
    </JetStreamContext.Provider>
  );
}

async function acceptIncoming(
  transport: WebTransport,
  handlers: JetStreamContextValue["handlers"],
) {
  const reader = transport.incomingBidirectionalStreams.getReader();
  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      // Each incoming stream: read Tversion, dispatch to the one matching handler
      handleStream(value, handlers).catch(() => {});
    }
  } finally {
    reader.releaseLock();
  }
}

async function handleStream(
  stream: {
    readable: ReadableStream<Uint8Array>;
    writable: WritableStream<Uint8Array>;
  },
  handlers: JetStreamContextValue["handlers"],
) {
  // Perform server-side version negotiation to identify the protocol
  const knownProtocols = new Set(handlers.keys());
  const accepted = await acceptVersion(
    stream.readable,
    stream.writable,
    knownProtocols,
  );

  // Look up the handler for this protocol
  const handler = handlers.get(accepted.protocolName);
  if (!handler) return;

  const codec = handler.createCodec();
  await serverLoop(codec, handler.dispatch, stream.readable, stream.writable);
}

export function useJetStreamStatus(): ConnectionState {
  const ctx = useContext(JetStreamContext);
  if (!ctx)
    throw new Error("useJetStreamStatus must be used within JetStreamProvider");
  return ctx.state;
}
