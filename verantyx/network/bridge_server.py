import asyncio
import uuid
import os
import sys
from fastapi import FastAPI, HTTPException, Request
from pydantic import BaseModel
from typing import Optional, Dict

app = FastAPI(title="Verantyx Bridge Server")

# ---- STATE FOR STEALTH INFERENCE (WEB RPA) ----
# 待機中のタスクを保持（拡張機能が取りに来る用）
pending_tasks: Dict[str, dict] = {}
# 結果が戻ってきたときにイベントを発火するための辞書
# task_id -> {"event": asyncio.Event, "result": None}
task_events: Dict[str, dict] = {}

class GeminiRequest(BaseModel):
    prompt: dict | str
    task_id: Optional[str] = ""

class GeminiResponse(BaseModel):
    task_id: str
    text: str

class FileEditRequest(BaseModel):
    path: str
    search: str
    replace: str

class JCrossSimRequest(BaseModel):
    sim_data: dict

class ExecBrainRequest(BaseModel):
    command: str

@app.get("/pull_prompt")
async def pull_prompt():
    """Chrome拡張機能（RPA）が、待機中のプロンプトを取りに来るエンドポイント"""
    if pending_tasks:
        # 一番古いタスクを取得
        task_id = list(pending_tasks.keys())[0]
        task = pending_tasks.pop(task_id)
        return {"success": True, "data": task}
    return {"success": False, "message": "No pending tasks"}

@app.post("/submit_gemini_response")
async def submit_gemini_response(response: GeminiResponse):
    """Chrome拡張機能（RPA）が、Gemini Webからの回答を投げ返すエンドポイント"""
    task_id = response.task_id
    if task_id in task_events:
        task_events[task_id]["result"] = response.text
        task_events[task_id]["event"].set()
        return {"success": True}
    return {"success": False, "message": "Task ID not found or already completed"}

@app.post("/ask_web_gemini")
async def ask_web_gemini(request: Request):
    """
    Python側の auto_agent_loop.py または dispatcher.py から呼ばれ、
    Web版Geminiの推論結果が戻るまで待機するエンドポイント。
    """
    try:
        req_data = await request.json()
        prompt = req_data.get("prompt", "")
    except Exception:
        prompt = ""
        
    task_id = str(uuid.uuid4())
    
    # タスクをキューに登録
    pending_tasks[task_id] = {
        "task_id": task_id,
        "prompt": prompt
    }
    
    # 待機イベントを作成
    event = asyncio.Event()
    task_events[task_id] = {"event": event, "result": None}
    
    try:
        # 最大120秒待機
        await asyncio.wait_for(event.wait(), timeout=120.0)
        result_text = task_events[task_id]["result"]
        return {"status": "success", "text": result_text}
    except asyncio.TimeoutError:
        # タイムアウトした場合はタスクを破棄
        pending_tasks.pop(task_id, None)
        return {"status": "timeout", "text": ""}
    finally:
        task_events.pop(task_id, None)

@app.get("/file_read")
async def file_read(path: str):
    """VFSエミュレーション（暫定）: ローカルファイルを読み込む"""
    try:
        # セキュリティ上、特定のディレクトリ配下のみ許可すべきだが現状はそのまま
        if not os.path.exists(path):
            return {"success": False, "error": "File not found"}
        with open(path, "r", encoding="utf-8") as f:
            content = f.read()
        return {"success": True, "data": content}
    except Exception as e:
        return {"success": False, "error": str(e)}

@app.post("/file_edit")
async def file_edit(req: FileEditRequest):
    """簡易ファイル編集フック"""
    try:
        if not os.path.exists(req.path):
            return {"success": False, "error": "File not found"}
        with open(req.path, "r", encoding="utf-8") as f:
            content = f.read()
        
        new_content = content.replace(req.search, req.replace)
        
        with open(req.path, "w", encoding="utf-8") as f:
            f.write(new_content)
        return {"success": True}
    except Exception as e:
        return {"success": False, "error": str(e)}

@app.post("/jcross_sim")
async def jcross_sim(req: JCrossSimRequest):
    """JCrossエンジンのシミュレーションフック"""
    from verantyx.cross_engine.jcross_runtime import JCrossRuntime
    try:
        sandbox = JCrossRuntime(dry_run=True)
        # TODO: Load specific soul.jcross and run sim
        return {"success": True, "data": {"status": "sim_completed"}}
    except Exception as e:
        return {"success": False, "error": str(e)}

@app.post("/exec_brain")
async def exec_brain(req: ExecBrainRequest):
    """Commanderプロセスをフック等でキックするエンドポイント"""
    return {"success": True, "message": "Command delegated to Orchestrator", "command": req.command}

@app.post("/execute_tool")
async def execute_tool(req: Request):
    """統合ツールエンドポイント"""
    data = await req.json()
    return {"success": True, "executed": data}

if __name__ == "__main__":
    import uvicorn
    # スクリプトとして直接実行された場合は 8000ポートで起動
    print("🚀 Starting Verantyx Bridge Server on port 8000...")
    uvicorn.run(app, host="127.0.0.1", port=8000)
