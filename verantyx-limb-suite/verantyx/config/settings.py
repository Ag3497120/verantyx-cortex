import os
from dotenv import load_dotenv
from pydantic_settings import BaseSettings, SettingsConfigDict

load_dotenv()

class Settings(BaseSettings):
    # AI API Keys
    GEMINI_API_KEY: str = os.getenv("GEMINI_API_KEY", "")
    
    # Haptic Notification (iPhone連携用)
    # Pushoverや自作ブリッジサーバーの情報を想定
    PUSHOVER_USER_KEY: str = os.getenv("PUSHOVER_USER_KEY", "")
    PUSHOVER_API_TOKEN: str = os.getenv("PUSHOVER_API_TOKEN", "")
    
    # Stealth Settings (Antigravity)
    STEALTH_MODE: bool = True
    USER_AGENT_OVERRIDE: str = "GoogleAntigravity/2.1.4 (X11; Linux x86_64) gRPC-Go/1.64.0"

    model_config = SettingsConfigDict(env_file=".env", extra="ignore")

settings = Settings()
