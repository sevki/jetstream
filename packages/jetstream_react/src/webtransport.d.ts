/**
 * r[impl jetstream.react.webtransport]
 * Minimal WebTransport type declarations for the JetStream React hooks.
 */
declare class WebTransport {
  constructor(url: string, options?: WebTransportOptions);
  readonly ready: Promise<void>;
  readonly closed: Promise<WebTransportCloseInfo>;
  readonly incomingBidirectionalStreams: ReadableStream<WebTransportBidirectionalStream>;
  close(closeInfo?: WebTransportCloseInfo): void;
  createBidirectionalStream(): Promise<WebTransportBidirectionalStream>;
}

interface WebTransportOptions {
  allowPooling?: boolean;
  congestionControl?: string;
  requireUnreliable?: boolean;
  serverCertificateHashes?: WebTransportHash[];
}

interface WebTransportCloseInfo {
  closeCode?: number;
  reason?: string;
}

interface WebTransportHash {
  algorithm: string;
  value: BufferSource;
}

interface WebTransportBidirectionalStream {
  readonly readable: ReadableStream<Uint8Array>;
  readonly writable: WritableStream<Uint8Array>;
}
