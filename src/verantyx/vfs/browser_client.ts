import { spawn, ChildProcess } from "child_process";
import { createInterface } from "readline";
import { existsSync } from "fs";

export interface BrowserResponse {
  status: "ok" | "error" | "hitl_done";
  message?: string;
  url?: string;
  title?: string;
  markdown?: string;
  elements?: Array<{
    id: number;
    element_type: string;
    label: string;
    href?: string;
    intent: string;
  }>;
  text?: string;
  token_estimate?: number;
}

/**
 * BrowserClient — Verantyx Wry Stealth Engine 
 *
 * Spawns `vx-browser --bridge` as a background process.
 * Under the hood, this loads a headless `wry::WebView` to extract DOM natively 
 * using WKWebView, bypassing Google Botguard.
 */
export class BrowserClient {
  private process: ChildProcess | null = null;
  private binPath: string;
  private requestQueue: any[] = [];
  private isReady = false;
  private hitlResolver: ((resp: BrowserResponse) => void) | null = null;

  private isVisible: boolean = false;

  constructor(binPath?: string) {
    this.binPath = binPath || "verantyx-browser/target/debug/vx-browser";
  }

  async restart(visible: boolean): Promise<void> {
      if (this.process) {
          this.process.kill();
          this.process = null;
          this.isReady = false;
          this.requestQueue = [];
      }
      this.isVisible = visible;
      await this.start();
  }

  async start(): Promise<void> {
    if (this.process) return;

    if (!existsSync(this.binPath)) {
        throw new Error(`vx-browser binary not found at ${this.binPath}. Run 'cargo build' first.`);
    }

    const args = ["--bridge"];
    if (this.isVisible) args.push("--visible");

    this.process = spawn(this.binPath, args, {
      stdio: ["pipe", "pipe", "inherit"],
    });

    const rl = createInterface({
      input: this.process.stdout!,
      terminal: false,
    });

    return new Promise((resolve, reject) => {
      this.process!.on("error", (err) => {
        console.error("[BrowserClient] Process error:", err);
        reject(err);
      });

      this.process!.on("exit", (code) => {
        console.warn(`[BrowserClient] Process exited with code ${code}`);
        this.process = null;
        this.isReady = false;
      });

      rl.on("line", (line) => {
        try {
          const resp = JSON.parse(line) as BrowserResponse;
          
          if (!this.isReady && resp.message?.includes("ready")) {
            this.isReady = true;
            resolve();
            return;
          }

          if (resp.status === "hitl_done") {
              if (this.hitlResolver) {
                  this.hitlResolver(resp);
                  this.hitlResolver = null;
              }
              return;
          }

          const nextResolver = this.requestQueue.shift();
          if (nextResolver) {
            nextResolver(resp);
          }
        } catch (e) {
          console.error("[BrowserClient] Failed to parse bridge response:", line);
        }
      });
    });
  }

  async command(cmd: any): Promise<BrowserResponse> {
    if (!this.isReady) {
      await this.start();
    }

    return new Promise((resolve) => {
      this.requestQueue.push(resolve);
      this.process!.stdin!.write(JSON.stringify(cmd) + "\n");
    });
  }

  async waitForAutoHarvest(): Promise<BrowserResponse> {
    if (!this.isReady) {
      await this.start();
    }
    return new Promise((resolve) => {
      this.hitlResolver = resolve;
    });
  }

  // MARK: - Browser Actions

  async navigate(url: string): Promise<BrowserResponse> {
    return this.command({ cmd: "navigate", url });
  }

  async click(id: number): Promise<BrowserResponse> {
    return this.command({ cmd: "click", id });
  }

  async type(id: number, text: string): Promise<BrowserResponse> {
    return this.command({ cmd: "type", id, text });
  }

  async submit(id: number): Promise<BrowserResponse> {
    return this.command({ cmd: "click", id });
  }

  async getPage(): Promise<BrowserResponse> {
    return this.command({ cmd: "get_page" });
  }

  async back(): Promise<BrowserResponse> {
    return this.command({ cmd: "back" });
  }

  async quit(): Promise<void> {
    if (this.process) {
      await this.command({ cmd: "quit" });
      this.process = null;
      this.isReady = false;
    }
  }
}
