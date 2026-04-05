# verantyx/haptic/bridge.py

import httpx
from verantyx.config.settings import settings
from verantyx.core.types import HapticPattern

async def send_haptic_to_iphone(pattern: HapticPattern, message: str = ""):
    """
    iPhoneへ触覚通知（振動信号）を送信する。
    """
    # 設定が空の場合はスキップ
    if not settings.PUSHOVER_USER_KEY or not settings.PUSHOVER_API_TOKEN:
        print(f"[!] Haptic keys not set. Skipping vibration: {pattern}")
        return False

    # パターンに応じた通知の優先度設定
    # Pushoverのpriority: 2 (緊急), 1 (高), 0 (普通), -1 (静か)
    priority = 0
    title = "Verantyx Event"

    if pattern == HapticPattern.DETECTION_ALERT:
        priority = 2  # 緊急（承認されるまで鳴り続ける設定などが可能）
        title = "⚠️ CRITICAL: DETECTION ALERT"
    elif pattern == HapticPattern.REQUIRE_APPROVAL:
        priority = 1
        title = "⏳ ACTION REQUIRED"
    elif pattern == HapticPattern.SUCCESS:
        priority = 0
        title = "✅ STEP SUCCESS"
    elif pattern == HapticPattern.MORSE_V:
        priority = 0
        title = "✨ Verantyx Active"

    # APIリクエストの組み立て
    payload = {
        "token": settings.PUSHOVER_API_TOKEN,
        "user": settings.PUSHOVER_USER_KEY,
        "message": f"{pattern.value}: {message}" if message else pattern.value,
        "title": title,
        "priority": priority,
        # 'sound' パラメータを使ってiPhone側の特定の振動/音をトリガー可能
        "sound": "vibrate_only" 
    }

    if priority == 2:
        payload["retry"] = 30  # 30秒ごとに再試行
        payload["expire"] = 3600 # 1時間で期限切れ

    try:
        async with httpx.AsyncClient() as client:
            response = await client.post(
                "https://api.pushover.net/1/messages.json",
                data=payload
            )
            if response.status_code == 200:
                print(f"[*] Haptic signal [{pattern}] sent to iPhone.")
                return True
            else:
                print(f"[!] Failed to send haptic: {response.text}")
                return False
    except Exception as e:
        print(f"[!] Haptic Bridge Error: {e}")
        return False
