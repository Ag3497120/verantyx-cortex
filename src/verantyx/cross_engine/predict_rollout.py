#!/usr/bin/env python3
import sys
import os
import json

engine_dir = "/Users/motonishikoudai/verantyx-cli/src/verantyx/cross_engine"
sys.path.insert(0, engine_dir)

from jcross_runtime import JCrossRuntime

def predict_rollout(json_str):
    try:
        vars_dict = json.loads(json_str)
    except json.JSONDecodeError as e:
        print(f"Error: Could not parse JSON array/dict -> {json_str} (err: {e})")
        return

    sandbox_engine = JCrossRuntime(dry_run=True)
    soul_path = os.path.join(engine_dir, "soul.jcross")
    if os.path.exists(soul_path):
        sandbox_engine.load(soul_path)
    else:
        print(f"Error: soul.jcross not found at {soul_path}")
        return

    virtual_rules = vars_dict.pop("仮想ルール", None)
    max_steps = vars_dict.pop("最大ステップ数", 10)

    for k, v in vars_dict.items():
        sandbox_engine.inject(k, v)

    if virtual_rules:
        sandbox_engine.inject_rules(virtual_rules)

    print("====== [JCross Sandbox Rollout Prediction Result] ======")
    print(f"シミュレーション開始 (最大ステップ数: {max_steps})\n")

    for step in range(1, max_steps + 1):
        action = sandbox_engine.decide()
        print(f"Step {step} | Action Decision: {action}")
        
        if action == -1:
            print(f"-> 【失敗】Step {step} で不正な行動（または停止）を返したためスタックしました。")
            break
            
        if hasattr(sandbox_engine, 'last_globals'):
            if sandbox_engine.last_globals.get("GAME_CLEAR"):
                print(f"-> 【成功】Step {step} でゲームクリア条件を獲得しました！生存テスト成功です。")
                break
            if sandbox_engine.last_globals.get("GAME_OVER"):
                print(f"-> 【失敗】Step {step} でゲームオーバーに抵触しました。")
                break

    print("\n========================================================")

if __name__ == "__main__":
    raw_args = " ".join(sys.argv[1:])
    if raw_args.startswith("--vars "):
        raw_args = raw_args[7:]
    raw_args = raw_args.strip().strip("'").strip('"')
    
    predict_rollout(raw_args)
