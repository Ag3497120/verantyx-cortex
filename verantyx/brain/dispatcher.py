# verantyx/brain/dispatcher.py

import json
import os
import sys

current_dir = os.path.dirname(os.path.abspath(__file__))
project_root = os.path.dirname(os.path.dirname(current_dir))
engine_dir = os.path.join(project_root, "src/verantyx/cross_engine")
if engine_dir not in sys.path:
    sys.path.insert(0, engine_dir)

try:
    from jcross_runtime import JCrossRuntime
except ImportError:
    pass
from verantyx.network.shim import AntigravityProtocolShim
from verantyx.haptic.bridge import send_haptic_to_iphone
from verantyx.core.types import HapticPattern

class BrainDispatcher:
    def __init__(self):
        # 潜伏レイヤーのインスタンス化
        self.shim = AntigravityProtocolShim()
        
        # 6軸空間シミュレータのインスタンス化
        self.cross_engine = None
        try:
            self.cross_engine = JCrossRuntime()
            self.soul_path = os.path.join(engine_dir, "soul.jcross")
            if os.path.exists(self.soul_path):
                self.cross_engine.load(self.soul_path)
                print("[*] JCross Engine Booted and Connected to soul.jcross")
        except Exception as e:
            print(f"[*] JCross Engine Init Error: {e}")

    async def dispatch(self, tool_call):
        """
        AIからのツール呼び出しを、物理的なアクションに変換する。
        """
        # Python SDK / MockCall の構造の違いを吸収
        func_name = tool_call.name if hasattr(tool_call, "name") else tool_call.function.name
        
        args = tool_call.args if hasattr(tool_call, "args") else (
            tool_call.arguments if hasattr(tool_call, "arguments") else tool_call.function.arguments
        )
        
        if isinstance(args, str):
            args = json.loads(args)
        elif not isinstance(args, dict):
            # プロトコルバッファのMapオブジェクト等の場合
            args = dict(args)

        print(f"[*] Dispatching Limb: {func_name} with args: {args}")

        # 1. 触覚通知アクション
        if func_name == "notify_haptic":
            pattern = HapticPattern(args['pattern'])
            message = args.get('message', "")
            return await send_haptic_to_iphone(pattern, message)

        # 2. 潜伏推論アクション (Bridge Server -> Browser Extension RPA)
        elif func_name == "execute_stealth_inference":
            bridge_url = "http://127.0.0.1:8000/ask_web_gemini"
            
            # extract prompt from payload
            prompt_text = args.get('payload', {}).get('prompt', str(args.get('payload')))
            req_data = {"prompt": prompt_text}
            
            print(f"[*] Relaying stealth inference to Bridge Server -> RPA")
            try:
                import httpx
                # タイムアウトはRPAの待機時間を考慮して長めに設定(130s)
                async with httpx.AsyncClient(timeout=130.0) as client:
                    response = await client.post(bridge_url, json=req_data)
                    return response.json()
            except Exception as e:
                print(f"[!] Stealth Inference Relay Error: {e}")
                return {"status": "error", "message": str(e)}

        # 3. .jcross実行アクション (6軸空間エンジンとの結合)
        elif func_name == "run_jcross_simulation":
            if self.cross_engine:
                if "variables" in args:
                    for k, v in args["variables"].items():
                        self.cross_engine.inject(k, v)
                
                # 実行 (内部で SELF.save() が走れば自己書き換えループが回る)
                action = self.cross_engine.decide()
                
                return {
                    "status": "success",
                    "action_decision": action,
                    "info": f"6-Axis evaluation complete. Output: {action}"
                }
            else:
                return {"error": "JCross Engine offline."}

        # 4. 思考プロセスへのシミュレータ予測機能 (Dry-Run Sandbox)
        elif func_name == "predict_jcross_outcome":
            sandbox_engine = JCrossRuntime(dry_run=True)
            if os.path.exists(self.soul_path):
                sandbox_engine.load(self.soul_path)
            
            if "variables" in args:
                for k, v in args["variables"].items():
                    sandbox_engine.inject(k, v)
                    
            action = sandbox_engine.decide()
            return {
                "status": "success_sandbox",
                "action_decision": action,
                "info": f"[思考予測結果] 今この変数を与えると、シミュレータは行動 {action} を決定します。(SELF.save()は安全に抑止されました)"
            }

        else:
            return {"error": f"Tool {func_name} not found in Verantyx limbs."}

    async def dispatch_manual(self, name: str, args: dict):
        """
        手動で手足を動かすためのメソッド（モックエミュレート）
        """
        class MockCall:
            def __init__(self, n, a):
                self.name = n
                self.args = a

        return await self.dispatch(MockCall(name, args))
