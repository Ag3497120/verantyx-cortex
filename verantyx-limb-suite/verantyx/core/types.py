from enum import Enum

class HapticPattern(str, Enum):
    MORSE_V = "morse_v"      # 起動・成功 (···—)
    SUCCESS = "success"      # 1回深い振動
    THINKING = "thinking"    # 小刻みな微振動
    REQUIRE_APPROVAL = "approval" # 2回はっきりとした振動
    DETECTION_ALERT = "alert" # 5回激しい振動 (緊急停止)

class AgentEvent(str, Enum):
    INITIALIZED = "initialized"
    THOUGHT_STARTED = "thought_started"
    JCROSS_EXECUTED = "jcross_executed"
    SUBMISSION_SENT = "submission_sent"
    ERROR_OCCURRED = "error_occurred"
