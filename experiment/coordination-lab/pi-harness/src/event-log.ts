import { appendFile, mkdir } from "node:fs/promises";
import { dirname } from "node:path";

export interface HarnessEvent {
  sequence: number;
  at: string;
  sessionId: string;
  actorId: string;
  type: string;
  payload: unknown;
}

export class EventLog {
  private sequence = 0;
  private tail: Promise<void> = Promise.resolve();

  constructor(
    private readonly path: string | undefined,
    private readonly sessionId: string,
    private readonly actorId: string,
  ) {}

  write(type: string, payload: unknown): Promise<void> {
    const event: HarnessEvent = {
      sequence: ++this.sequence,
      at: new Date().toISOString(),
      sessionId: this.sessionId,
      actorId: this.actorId,
      type,
      payload,
    };
    if (!this.path) return Promise.resolve();
    this.tail = this.tail.then(async () => {
      await mkdir(dirname(this.path!), { recursive: true });
      await appendFile(this.path!, `${JSON.stringify(event)}\n`, "utf8");
    });
    return this.tail;
  }

  flush(): Promise<void> {
    return this.tail;
  }
}
