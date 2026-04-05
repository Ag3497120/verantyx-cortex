# verantyx/network/profiles.py

from dataclasses import dataclass, field
from typing import Dict

@dataclass
class StealthProfile:
    name: str
    user_agent: str
    base_headers: Dict[str, str] = field(default_factory=dict)
    # HTTP/2 や TLS の挙動を模倣するためのヒント
    impersonate: str = "chrome120" 

# 2026年3月現在の Google Antigravity 公式プロファイル
OFFICIAL_ANTIGRAVITY_PROFILE = StealthProfile(
    name="antigravity_v2_1",
    user_agent="GoogleAntigravity/2.1.4 (X11; Linux x86_64) gRPC-Go/1.64.0",
    base_headers={
        "X-Goog-Api-Client": "gl-python/3.12.2 grpc/1.62.0",
        "X-Antigravity-Source": "cli-native",
        "X-Antigravity-Internal-ID": "0x4B4F444149", # あなたのIDの16進数表現などを模倣
        "Te": "trailers",
        "Content-Type": "application/grpc",
    }
)
