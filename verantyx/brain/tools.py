# verantyx/brain/tools.py

VERANTYX_TOOLS = [
    {
        "name": "predict_jcross_outcome",
        "description": "現在のシミュレータ層を仮想実行し、行動結果を能動的に「事前予測」する。SELF.save()等の状態汚染は発生しないため、思考プロセス中のWhat-If検証に最適。",
        "parameters": {
            "type": "OBJECT",
            "properties": {
                "variables": {"type": "OBJECT", "description": "仮想環境に注入する変数の辞書（例: {'フェーズ': '探索', '行動キュー': [2]}）"}
            },
            "required": ["variables"]
        }
    },
    {
        "name": "notify_haptic",
        "description": "ユーザーのiPhoneに特定の振動パターンを送る。進捗報告や緊急停止の際に使用。",
        "parameters": {
            "type": "OBJECT",
            "properties": {
                "pattern": {
                    "type": "STRING", 
                    "enum": ["morse_v", "success", "thinking", "approval", "alert"],
                    "description": "振動の種類（モールス信号、成功、思考中、承認待ち、警告）"
                },
                "message": {"type": "STRING", "description": "通知に表示する短いメッセージ"}
            },
            "required": ["pattern"]
        }
    },
    {
        "name": "execute_stealth_inference",
        "description": "Google Antigravityプロトコルを装って推論リクエストを送信する。検知を回避しながら外部知能を利用。",
        "parameters": {
            "type": "OBJECT",
            "properties": {
                "payload": {"type": "OBJECT", "description": "送信する推論リクエストの内容（JSON形式）"}
            },
            "required": ["payload"]
        }
    },
    {
        "name": "run_jcross_simulation",
        "description": "生成した.jcrossコードをCROSSエンジンで実行し、ARCパズルの推論結果を検証する。",
        "parameters": {
            "type": "OBJECT",
            "properties": {
                "code": {"type": "STRING", "description": "実行する.jcross DSLコード"},
                "grid_id": {"type": "STRING", "description": "検証対象のグリッド識別子"}
            },
            "required": ["code"]
        }
    },
    {
        "name": "predict_jcross_outcome",
        "description": "現在のシミュレータ層を仮想実行し、行動結果を能動的に「事前予測」する。SELF.save()等の状態汚染は発生しないため、思考プロセス中のWhat-If検証に最適。",
        "parameters": {
            "type": "OBJECT",
            "properties": {
                "variables": {"type": "OBJECT", "description": "仮想環境に注入する変数の辞書（例: {\"フェーズ\": \"探索\", \"行動キュー\": [2]}）"}
            },
            "required": ["variables"]
        }
    }
]
