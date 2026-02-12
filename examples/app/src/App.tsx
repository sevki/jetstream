// r[impl jetstream.react.example]
// r[impl jetstream.webtransport.example]
import { useState } from "react";
import {
  JetStreamProvider,
  useJetStream,
  useJetStreamStatus,
  useRPC,
} from "@sevki/jetstream-react";
import {
  EchoHttpClient,
  rmessageDecode,
  PROTOCOL_VERSION,
  PROTOCOL_NAME,
} from "./generated/echohttp_rpc.js";
import JetStreamLogo from "../../../logo/JetStream.png";

const SERVER_URL = `https://127.0.0.1:4433/${PROTOCOL_NAME}`;

function EchoDemo() {
  const status = useJetStreamStatus();
  const echo = useJetStream(EchoHttpClient, rmessageDecode, PROTOCOL_VERSION);
  const [message, setMessage] = useState("hello");
  const [sum, setSum] = useState<number | null>(null);

  const { data, error, isLoading } = useRPC(
    () => (echo ? echo.ping(message) : Promise.resolve("")),
    [echo, message],
  );

  return (
    <div>
      <img src={JetStreamLogo} alt="JetStream Logo" width={200} />
      <h1>JetStream Echo Demo</h1>
      <div data-testid="connection-status" className={`status ${status}`}>
        {status}
      </div>
      <div data-testid="protocol-version">Protocol: {PROTOCOL_VERSION}</div>

      <h2>Ping (echo)</h2>
      <input
        data-testid="ping-input"
        type="text"
        value={message}
        onChange={(e) => setMessage(e.target.value)}
        placeholder="Type a message..."
      />
      {isLoading && <div data-testid="ping-loading">Loading...</div>}
      {error && <div data-testid="ping-error">Error: {error.message}</div>}
      {data && <div data-testid="ping-result">Echo: {data}</div>}

      <h2>Add</h2>
      <button
        data-testid="add-btn"
        disabled={!echo}
        onClick={async () => {
          if (echo) {
            const result = await echo.add(2, 3);
            setSum(result);
          }
        }}
      >
        Add 2 + 3
      </button>
      {sum !== null && <div data-testid="add-result">Sum: {sum}</div>}
    </div>
  );
}

export default function App() {
  return (
    <JetStreamProvider url={SERVER_URL}>
      <EchoDemo />
    </JetStreamProvider>
  );
}
