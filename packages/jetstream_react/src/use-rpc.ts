/**
 * r[impl jetstream.react.use-rpc]
 * r[impl jetstream.react.use-rpc.deps]
 * r[impl jetstream.react.use-rpc.stale]
 * r[impl jetstream.react.use-rpc.refetch]
 */
import { useCallback, useEffect, useRef, useState } from 'react';

export interface UseRPCResult<T> {
  data: T | undefined;
  error: Error | undefined;
  isLoading: boolean;
  refetch: () => void;
}

/**
 * Provides a reactive wrapper around a single upstream RPC call.
 * Returns { data, error, isLoading, refetch } and triggers the call
 * on mount or when dependencies change.
 */
export function useRPC<T>(
  fn: () => Promise<T>,
  deps: readonly unknown[],
): UseRPCResult<T> {
  const [data, setData] = useState<T | undefined>(undefined);
  const [error, setError] = useState<Error | undefined>(undefined);
  const [isLoading, setIsLoading] = useState(true);
  const versionRef = useRef(0);

  const execute = useCallback(() => {
    const version = ++versionRef.current;
    setIsLoading(true);
    setError(undefined);

    fn()
      .then((result) => {
        // Only update if this is still the latest request
        if (versionRef.current === version) {
          setData(result);
          setIsLoading(false);
        }
      })
      .catch((err) => {
        if (versionRef.current === version) {
          setError(err instanceof Error ? err : new Error(String(err)));
          setIsLoading(false);
        }
      });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);

  useEffect(() => {
    execute();
  }, [execute]);

  return { data, error, isLoading, refetch: execute };
}
