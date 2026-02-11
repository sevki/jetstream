/**
 * r[impl jetstream.rpc.ts.tag-pool]
 */
export class TagPool {
  private available: number[] = [];
  private maxTag: number;

  constructor(maxConcurrent: number = 256) {
    this.maxTag = maxConcurrent;
    for (let i = maxConcurrent; i >= 1; i--) {
      this.available.push(i);
    }
  }

  acquire(): number | null {
    return this.available.pop() ?? null;
  }

  release(tag: number): void {
    this.available.push(tag);
  }
}
