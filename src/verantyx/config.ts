/**
 * Verantyx Agent Configuration
 *
 * Defines model assignments for Commander/Worker/Scout roles.
 * Both Commander and Worker default to Opus for maximum reasoning quality.
 */

import { readFileSync, existsSync } from "fs";
import { join } from "path";

export interface RoninAgentConfig {
  agents: {
    systemLanguage: "en" | "ja";
    commanderModel: string;
    workerModel: string;
    scoutModel: string;
    cloudFallbackMode: "api" | "browser_hitl";
  };
  providers: {
    anthropic: {
      apiKey?: string;
      oauthToken?: string;
    };
  };
}

const DEFAULT_CONFIG: RoninAgentConfig = {
  agents: {
    systemLanguage: "en",
    commanderModel: "claude-opus-4-6",
    workerModel: "claude-sonnet-4-6",
    scoutModel: "claude-sonnet-4-6",
    cloudFallbackMode: "browser_hitl", // Ultimate safety default
  },
  providers: {
    anthropic: {},
  },
};

/**
 * Load Ronin config from:
 * 1. RONIN_CONFIG env var (JSON file path)
 * 2. ~/.ronin/config.json
 * 3. OpenClaw's ~/.openclaw/openclaw.json (extract API key)
 * 4. Fallback to defaults
 */
export function loadConfig(): RoninAgentConfig {
  const config = { ...DEFAULT_CONFIG };

  // Try RONIN_CONFIG env
  const envPath = process.env.RONIN_CONFIG;
  if (envPath && existsSync(envPath)) {
    try {
      const raw = JSON.parse(readFileSync(envPath, "utf-8"));
      if (raw.agents) {
        config.agents = { ...config.agents, ...raw.agents };
      }
      if (raw.providers) {
        config.providers = { ...config.providers, ...raw.providers };
      }
    } catch { /* ignore parse errors */ }
  }

  // Try ~/.ronin/config.json
  const homeConfig = join(
    process.env.HOME || "",
    ".ronin",
    "config.json"
  );
  if (existsSync(homeConfig)) {
    try {
      const raw = JSON.parse(readFileSync(homeConfig, "utf-8"));
      if (raw.agents) {
        config.agents = { ...config.agents, ...raw.agents };
      }
      if (raw.providers?.anthropic) {
        config.providers.anthropic = {
          ...config.providers.anthropic,
          ...raw.providers.anthropic,
        };
      }
    } catch { /* ignore */ }
  }

  // Try OpenClaw's config for API key
  const openclawConfig = join(
    process.env.HOME || "",
    ".openclaw",
    "openclaw.json"
  );
  if (existsSync(openclawConfig) && !config.providers.anthropic.apiKey) {
    try {
      const raw = JSON.parse(readFileSync(openclawConfig, "utf-8"));
      // OpenClaw stores API key in various locations
      if (raw.anthropic?.api_key) {
        config.providers.anthropic.apiKey = raw.anthropic.api_key;
      }
      if (raw.provider === "anthropic" && raw.apiKey) {
        config.providers.anthropic.apiKey = raw.apiKey;
      }
    } catch { /* ignore */ }
  }

  // Environment variable override
  if (process.env.ANTHROPIC_API_KEY) {
    config.providers.anthropic.apiKey = process.env.ANTHROPIC_API_KEY;
  }

  // Model overrides from env
  if (process.env.RONIN_COMMANDER_MODEL) {
    config.agents.commanderModel = process.env.RONIN_COMMANDER_MODEL;
  }
  if (process.env.RONIN_WORKER_MODEL) {
    config.agents.workerModel = process.env.RONIN_WORKER_MODEL;
  }
  if (process.env.RONIN_SCOUT_MODEL) {
    config.agents.scoutModel = process.env.RONIN_SCOUT_MODEL;
  }

  return config;
}

/**
 * Resolve the API key from provider config.
 * Supports both direct API key and OAuth token.
 */
export function resolveProviderApiKey(
  provider: RoninAgentConfig["providers"]["anthropic"]
): string {
  return provider.apiKey || provider.oauthToken || process.env.ANTHROPIC_API_KEY || "";
}
