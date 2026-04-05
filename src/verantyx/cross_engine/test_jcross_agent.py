#!/usr/bin/env python3
"""JCrossエージェントの実戦テスト用スクリプト（ARC-AGI-3）"""
import sys, os, time

# ARC-AGI-3 のパスを追加（ユーザーの環境に合わせて調整）
ARC_DIR = "/Users/motonishikoudai/verantyx_v6/arc-agi-3/ARC-AGI-3-Agents"
sys.path.insert(0, ARC_DIR)

from dotenv import load_dotenv
load_dotenv(dotenv_path=os.path.join(ARC_DIR, ".env.example"))
load_dotenv(dotenv_path=os.path.join(ARC_DIR, ".env"), override=True)

try:
    from arc_agi import Arcade
    from arcengine import GameAction
except ImportError:
    print("Error: arc_agi モジュールが見つかりません。")
    print(f"venv をアクティベートするか、PYTHONPATH に {ARC_DIR} を追加して実行してください。")
    sys.exit(1)

# 自作のJCrossRuntimeを読み込み
engine_dir = "/Users/motonishikoudai/verantyx-cli/src/verantyx/cross_engine"
sys.path.insert(0, engine_dir)
from jcross_runtime import JCrossRuntime

ROOT_URL = "https://three.arcprize.org"
test_games = ["ls20", "lp85", "m0r0", "ft09", "tr87"]

# Arcade 初期化バグ修正版
api_key = os.environ.get("ARC_API_KEY", "")
arcade = Arcade(arc_base_url=ROOT_URL, arc_api_key=api_key)

print(f"Arcade initialized with base URL: {ROOT_URL}")

try:
    scorecard = arcade.create_scorecard()
    print(f"Scorecard: {scorecard}")
except Exception as e:
    print(f"Failed to create scorecard. Error: {e}")
    sys.exit(1)

total_levels = 0
soul_path = os.path.join(engine_dir, "soul.jcross")

for game_name in test_games:
    print(f"\n--- Starting Game: {game_name} ---")
    
    agent_brain = JCrossRuntime(dry_run=False)
    if not agent_brain.load(soul_path):
        print(f"Failed to load soul.jcross. Skipping {game_name}.")
        continue

    try:
        env = arcade.make(game_name, scorecard_id=scorecard)
        frame = env.reset()
    except Exception as e:
        print(f"Failed to get environment for {game_name}: {e}")
        continue
        
    actions = 0
    max_actions = 100
    levels = 0
    
    while actions < max_actions:
        # 視覚データの注入 (Phase A実装)
        if hasattr(frame, "grid"):
            agent_brain.inject("FRONT.grid", frame.grid)
            
        agent_brain.inject("フェーズ", "探索")
        agent_brain.inject("行動キュー", [])
        
        # JCross脳に行動を決定させる
        action_decision = agent_brain.decide()
        
        if action_decision == -1:
            print(f"[{game_name}] エージェントがフリーズしました (Return -1).")
            break
            
        print(f"[{game_name}] Step {actions}: JCross chose action {action_decision}")
        try:
            env_action = next((a for a in GameAction if a.value == int(action_decision)), None)
            if env_action is None:
                raise ValueError(f"Action {action_decision} is not mapped in arcengine.GameAction")
            frame = env.step(env_action)
        except Exception as e:
            print(f"Environment Error: {e}")
            break
            
        actions += 1
        
        if getattr(frame, "state", "") == "GAME_OVER":
            print(f"-> GAME OVER on step {actions}")
            break
            
    levels = getattr(frame, 'levels_completed', 0)
    total_levels += levels
    print(f"{game_name} completed: levels={levels} actions={actions}")

print(f"\nJCross Engine total: {total_levels} levels from {len(test_games)} games")
