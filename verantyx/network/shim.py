# verantyx/network/shim.py

import asyncio
import random
import httpx
from datetime import datetime
from verantyx.config.settings import settings
from .profiles import OFFICIAL_ANTIGRAVITY_PROFILE
from verantyx.core.types import HapticPattern

class AntigravityProtocolShim:
    def __init__(self, profile=OFFICIAL_ANTIGRAVITY_PROFILE):
        self.profile = profile
        self.client = httpx.AsyncClient(
            http2=True, # 公式はgRPCベースなのでHTTP/2が必須
            timeout=60.0,
            follow_redirects=True
        )

    async def _apply_behavioral_jitter(self):
        """公式ツール特有の『不規則な処理待ち』を再現し、bot検知を回避"""
        # 0.2秒〜1.5秒の間でランダムに待機
        jitter = random.uniform(0.2, 1.5)
        await asyncio.sleep(jitter)

    async def request(self, method: str, url: str, **kwargs):
        """
        全通信を公式プロトコルに擬態させて送信する。
        """
        if not settings.STEALTH_MODE:
            # 隠密モードOFFの場合はそのまま送信
            return await self.client.request(method, url, **kwargs)

        # 1. ヘッダーの擬態
        custom_headers = {**self.profile.base_headers, **kwargs.get("headers", {})}
        custom_headers["User-Agent"] = self.profile.user_agent
        kwargs["headers"] = custom_headers

        # 2. 挙動の擬態（ジッター挿入）
        await self._apply_behavioral_jitter()

        try:
            print(f"[{datetime.now().strftime('%H:%M:%S')}] [STEALTH] Sending mimicking request...")
            response = await self.client.request(method, url, **kwargs)
            
            # ステータスコードによる自己診断
            if response.status_code == 429: # Rate Limit
                # iPhoneへSOS振動を送るフラグを立てる（後ほど実装）
                print("[!] CRITICAL: Rate limit detected. Impersonation might be failing.")
                
            return response
            
        except Exception as e:
            print(f"[!] Network Error: {e}")
            raise e

    async def close(self):
        await self.client.aclose()
