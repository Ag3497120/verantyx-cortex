#!/usr/bin/env python3
"""
完全自律進化エージェント（Auto-Evolution Closed-Loop）
JCrossエンジンがスタック（Action: -1）した際、自動でGemini APIをコールして
ワンライナー関数ルールを生成させ、 Sandbox内でテストした上で、
成功すれば教え込み（teach.py互換処理）、即座にゲームを再開します。
"""

import sys, os, time
import json
import urllib.request
from typing import Optional

# ARC-AGI-3 のパスを追加
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
    sys.exit(1)

# 自作のJCrossRuntimeを読み込み
engine_dir = "/Users/motonishikoudai/verantyx-cli/src/verantyx/cross_engine"
sys.path.insert(0, engine_dir)
from jcross_runtime import JCrossRuntime
from cross_space import CrossSpace, Experience
from shape_analyzer import ShapeAnalyzer
from rule_mixer import RuleMixer

GEMINI_API_KEY = "AIzaSyATPOY0fmk94_bOWvkj13tvXGIegyjsZKE" # User Configured Gemini Key
GEMINI_MODEL = "gemini-2.5-pro"

# CrossEngine の Sensor と World のロード
import importlib.util
def load_cross_module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod

try:
    sensor_mod = load_cross_module('cross_sensor', os.path.join(ARC_DIR, 'agents/cross_engine/cross_sensor.py'))
    sim_mod = load_cross_module('simulator', os.path.join(ARC_DIR, 'agents/cross_engine/simulator.py'))
    CrossSensor = sensor_mod.CrossSensor
    CrossWorld = sim_mod.CrossWorld
except Exception as e:
    print(f"Failed to load CrossSensor/CrossWorld: {e}")
    CrossSensor = None
    CrossWorld = None

def call_gemini_api(prompt: str, system_prompt: str) -> Optional[str]:
    """Webブラウザ拡張機能(verantyx_eye)経由でWeb版Geminiを叩く（RPA）"""
    url = "http://127.0.0.1:8000/ask_web_gemini"
    
    # SYSTEM PROMPT はWeb版Geminiでは直接渡せないのでプロンプトの先頭にマージする
    combined_prompt = f"【システム指示】\n{system_prompt}\n\n【状況】\n{prompt}"
    payload = {
        "prompt": combined_prompt,
        "task_id": ""  # Bridge Serverが自動生成する
    }
    
    data = json.dumps(payload).encode('utf-8')
    req = urllib.request.Request(url, data=data, headers={"Content-Type": "application/json"})
    
    print("🚀 [Web RPA] Dispatching prompt to your active Gemini browser window...")
    try:
        # この通信はGeminiが返答を書き終わるまで（最大120秒）待機する
        with urllib.request.urlopen(req, timeout=120) as response:
            result = json.loads(response.read().decode('utf-8'))
            if result.get("status") == "success":
                print("📩 [Web RPA] Reply received from browser!")
                return result.get("text")
            else:
                print(f"❌ Bridge returned failure: {result}")
                return None
    except Exception as e:
        print(f"❌ Web Integration Bridge Error: {e}")
        return None

def extract_jcross_code(text: str) -> Optional[str]:
    """Geminiの応答からJCrossコード（関数）を抽出"""
    import re
    # 1. バッククォートブロックがあれば最優先で取得
    match = re.search(r'```(?:jcross|text)?\n(.*?)\n```', text, re.DOTALL)
    if match:
        content = match.group(1).strip()
        # 複数行ある可能性を考慮して1行化
        return content.replace('\n', ' ')

    # 2. ブロックがない場合、`関数 ` から文末または最後の `}` までを取得
    match = re.search(r'(?:関数|PATTERN|FUNCTION)\s+[\w_]+\(.*?\)\s*\{.*\}', text.replace('\n', ' '))
    if match:
        return match.group(0)
        
    return None

def test_rule_in_sandbox(soul_path: str, visual_grid: list, p_pos: list, t_pos: list, anomalies: list, phase: str, rule_str: str) -> bool:
    """新ルールがSandboxでスタックせずに行動を返すかテスト"""
    print(f"🔬 [Sandbox] Testing virtual rule: {rule_str[:50]}...")
    sandbox = JCrossRuntime(dry_run=True)
    if not sandbox.load(soul_path):
        return False
        
    sandbox.inject("FRONT.grid", visual_grid)
    sandbox.inject("PLAYER", p_pos)
    sandbox.inject("TARGET", t_pos)
    sandbox.inject("ANOMALIES", anomalies)
    sandbox.inject("フェーズ", phase)
    sandbox.inject("行動キュー", [])
    
    sandbox.inject_rules(rule_str)
    
    # 複数回ループでスタックしないか確認する
    for step in range(3):
        try:
            action = sandbox.decide()
            if action == -1:
                print(f"⚠️ Sandbox failed on step {step+1}: Returned -1")
                return False
        except Exception as e:
            print(f"⚠️ Sandbox Exception: {e}")
            return False
            
    # Survival test passed
    print(f"✅ Sandbox test passed! Rule is viable.")
    return True

def commit_rule_to_soul(soul_path: str, rule_str: str, explanation: str):
    """teach.py 互換: soul.jcross の末尾に追記して永続化"""
    import datetime
    import uuid
    timestamp = datetime.datetime.now().strftime("%Y%m%d_%H%M%S")
    rule_id = str(uuid.uuid4())[:12]
    
    block = f"\n\n// [RULE:AutoEvolution_{timestamp}_{rule_id} START]\n"
    block += f"// AI Explanation: {explanation}\n"
    block += f"{rule_str}\n"
    block += f"// [RULE:AutoEvolution_{timestamp}_{rule_id} END]\n"
    
    with open(soul_path, "a", encoding="utf-8") as f:
        f.write(block)
    print(f"💾 Permanently committed new rule to {soul_path}")

def get_current_cortex(soul_path: str, rule_id: str) -> str:
    """現在のAI_CORTEXの中身を取得する（LLMのプロンプトに渡してコンテキストを維持するため）"""
    try:
        with open(soul_path, "r", encoding="utf-8") as f:
            source = f.read()
        start = f"// [RULE:{rule_id} START]"
        end = f"// [RULE:{rule_id} END]"
        s = source.find(start)
        e = source.find(end)
        if s != -1 and e != -1:
            return source[s + len(start):e].strip()
    except Exception:
        pass
    return "関数 AIによる推論ルール() { 返す -1 }"

def extract_experiences(old_grid, new_grid, old_p_pos, new_p_pos, action, frame_idx) -> list:
    experiences = []
    if not old_grid or not new_grid: return experiences
    
    changed_colors = set()
    for r in range(len(old_grid)):
        for c in range(len(old_grid[0])):
            if r < len(new_grid) and c < len(new_grid[0]):
                if old_grid[r][c] != new_grid[r][c]:
                    changed_colors.add(old_grid[r][c])
                    changed_colors.add(new_grid[r][c])
                    
    pos_tuple = tuple(old_p_pos) if old_p_pos and len(old_p_pos) == 2 else (0,0)
    
    if not changed_colors and old_p_pos == new_p_pos and action in [0,1,2,3]:
        # Blocked (tried to move but didn't, and grid didn't change)
        blocked_colors = set()
        if old_p_pos and len(old_p_pos) == 2:
            r, c = old_p_pos
            if action == 0 and r > 0: blocked_colors.add(old_grid[r-1][c])
            elif action == 1 and c < len(old_grid[0])-1: blocked_colors.add(old_grid[r][c+1])
            elif action == 2 and r < len(old_grid)-1: blocked_colors.add(old_grid[r+1][c])
            elif action == 3 and c > 0: blocked_colors.add(old_grid[r][c-1])
        
        experiences.append(Experience(
            position=pos_tuple,
            colors=blocked_colors if blocked_colors else set([old_grid[pos_tuple[0]][pos_tuple[1]]] if len(old_grid)>pos_tuple[0] else []),
            event='blocked',
            action=action,
            frame=frame_idx,
            grid_state_hash=hash(str(old_grid))
        ))
        
    elif old_p_pos != new_p_pos:
        experiences.append(Experience(
            position=pos_tuple,
            colors=changed_colors if changed_colors else {0},
            event='passed',
            action=action,
            frame=frame_idx,
            grid_state_hash=hash(str(old_grid))
        ))
        
    elif changed_colors:
        event_type = 'opened' if (5 in changed_colors or 0 in changed_colors) else 'changed'
        experiences.append(Experience(
            position=pos_tuple,
            colors=changed_colors,
            event=event_type,
            action=action,
            frame=frame_idx,
            grid_state_hash=hash(str(old_grid))
        ))
        
    return experiences

def main():
    ROOT_URL = "https://three.arcprize.org"
    test_games = ["ls20", "lp85", "m0r0", "ft09", "tr87"]

    api_key = os.environ.get("ARC_API_KEY", "")
    arcade = Arcade(arc_base_url=ROOT_URL, arc_api_key=api_key)
    
    print(f"Arcade initialized with base URL: {ROOT_URL}")

    try:
        scorecard = arcade.create_scorecard()
    except Exception as e:
        print(f"Failed to create scorecard. Error: {e}")
        return

    total_levels = 0
    soul_path = os.path.join(engine_dir, "soul.jcross")
    
    SYSTEM_PROMPT = (
        "あなたはJCross言語のマスターAIであり、ARC-AGI-3の「世界モデル（Model-Based RL）」を構築する自律推論エージェント（科学者）です。\n"
        "【タスク】\n"
        "1. 盤面情報から「この世界の物理法則（何色に触れると弾かれるか・通れるか）」の仮説を立ててください。\n"
        "2. その仮説を『検証するため』、あるいは自信があるなら『クリアするため』のゴール位置(TARGET等)を定め、JCross行動ルールを生成してください。\n"
        "【厳守事項】\n"
        "1. 出力フォーマットは、説明文章（仮説）のあとに必ず `関数 AIによる推論ルール() { ... }` を書き出してください。\n"
        "2. 直接アクション（0:上 などの数値）を返すのではなく、目標座標や壁の色を指定して `返す 経路探索(TARGET, [障害物となる色番号の配列])` を使って行動を決定してください。\n"
        "3. どの条件にも合致しない場合は、必ず最後に `返す -1` を記述してください。絶対に `返す 0` などの無関係なデフォルト行動を書かないでください。\n"
        "【組み込み関数（最重要）】\n"
        "- 経路探索(目標座標配列, 障害物の色配列) : 指定した目標へ、避けたい色を避けながらA*アルゴリズムで最も効率の良い『次の1手』を自動計算します。\n"
        "  例: もし LENGTH(PLAYER)>0 そして LENGTH(TARGET)>0 { 返す 経路探索(TARGET, [1, 5]) }\n"
        "- 位置検索(色番号) : 盤面から指定した色の座標[Y, X]を検索して返します。鍵やスイッチに向かいたい場合に使ってください。\n"
        "  例: カギ = 位置検索(3)\n"
        "【利用可能な変数】\n"
        "- FRONT.grid : 画面上の全体セル配列\n"
        "- PLAYER : [Y, X]形式の現在座標。見つからない場合は空配列 []\n"
        "- TARGET : [Y, X]形式の目標座標\n"
        "- ANOMALIES : 異常物体のリスト"
    )

    for game_name in test_games:
        print(f"\n======================================")
        print(f"🛸 Starting Game: {game_name}")
        
        # ルールロード（自動更新される可能性があるのでループごと＆復帰ごとにリロード）
        def load_brain():
            brain = JCrossRuntime(dry_run=False)
            brain.load(soul_path)
            return brain
            
        agent_brain = load_brain()

        try:
            env = arcade.make(game_name, scorecard_id=scorecard)
            frame = env.reset()
        except Exception as e:
            print(f"Failed to get environment for {game_name}: {e}")
            continue
            
        actions = 0
        max_actions = 100
        sensor = CrossSensor() if CrossSensor else None
        cross_space = CrossSpace()
        
        # ----------------------------------------------------
        # [Phase 1] 俯瞰と関係グラフ構築 (Relational Graph Object Abstraction)
        # ----------------------------------------------------
        initial_grid = getattr(frame, "grid", [])
        if not initial_grid and hasattr(frame, 'frame') and hasattr(frame.frame, '__getitem__'):
             initial_grid = frame.frame[0].tolist() # Numpy array to list fallback

        identified_patterns = []
        target_shape = None
        
        if initial_grid:
            relational_graph = ShapeAnalyzer.build_relational_graph(initial_grid)
            identified_patterns = [f"{n['type']} Size:{n['size']} Col:{n['color']} BBox:{n['bbox']}" for n in relational_graph["nodes"]]
            print(f"👁️  [Phase 1] 関係グラフ構築: {len(relational_graph['nodes'])}個のノードと{len(relational_graph['edges'])}本の因果エッジ(Adjacency/Containment)を抽出")
            
        # 物理Probeフェーズを完全に撤廃 (Active Inferenceへの移行)
        print("🤖 [Phase 1.5] 物理Probeをスキップ -> 仮想検証(仮説構築)へ直行します。")

        print("🧠 [Phase 3 & 4] 思考と探索ループ（JCrossエンジン起動）")
        # ----------------------------------------------------
        
        last_grid_hash = None
        stagnation_counter = 0
        last_failed_rule = None
        evolution_attempts = 0

        while actions < max_actions:
            # 視覚と意味メタデータの注入
            grid_data = getattr(frame, "grid", [])
            phase = "探索"
            
            p_pos = []
            t_pos = []
            anomalies_summary = []
            
            if grid_data and CrossWorld and sensor:
                world = CrossWorld(grid_data)
                snap = sensor.observe(grid_data)
                
                if world.player_pos:
                    p_pos = list(world.player_pos)
                if world.lock_pos:
                    t_pos = list(world.lock_pos)
                
                for a in snap.anomalies[:3]:
                    anomalies_summary.append(f"{a.get('type')} at {a.get('position')}")
                
            agent_brain.inject("FRONT.grid", grid_data)
            agent_brain.inject("フェーズ", phase)
            agent_brain.inject("行動キュー", [])
            agent_brain.inject("PLAYER", p_pos)
            agent_brain.inject("TARGET", t_pos)
            agent_brain.inject("ANOMALIES", anomalies_summary)
            
            # 決断
            action_decision = agent_brain.decide()
            
            # ----------------------------------------------------
            # Stagnation Monitor (硬直判定)
            # ----------------------------------------------------
            curr_hash = hash(str(grid_data))
            if last_grid_hash == curr_hash and action_decision != 5:
                stagnation_counter += 1
            else:
                stagnation_counter = 0
                last_grid_hash = curr_hash

            if stagnation_counter >= 3:
                print(f"⚠️ [Stagnation Monitor] 盤面が3ターン変化しませんでした（ルール破綻）。AI_CORTEXを破棄して進化を強制します。")
                last_failed_rule = get_current_cortex(soul_path, "AI_CORTEX")
                agent_brain.rewrite_rule("AI_CORTEX", "関数 AIによる推論ルール() {\n    返す -1\n}")
                action_decision = -1
                stagnation_counter = 0
            # ----------------------------------------------------
            
            if action_decision == -1:
                print(f"🛑 [{game_name}] Agent stuck (Return -1) at step {actions}. Triggering Auto-Evolution...")
                # JCrossプロンプト構築
                current_cortex = get_current_cortex(soul_path, "AI_CORTEX")
                
                # CrossSpace から共鳴と衝動を取得
                biological_intents = []
                if cross_space.experiences:
                    latest_exp = cross_space.experiences[-1]
                    resonances = cross_space.resonate(latest_exp)
                    for score, past_exp in resonances[:3]:
                        intent = cross_space.collide(latest_exp, past_exp)
                        if intent:
                            biological_intents.append(f"【CrossSpaceからの自然衝動】 {intent.reason}")
                
                intents_text = "\n".join(biological_intents) if biological_intents else "【CrossSpaceからの自然衝動】 関連する強いブロック衝突の経験はありませんでした。"
                
                # 物理プローブの履歴を構築
                exp_history = []
                for exp in cross_space.experiences[-10:]: # 直近10件
                    exp_history.append(f"[Action:{exp.action}] => {exp.event} (関与色:{list(exp.colors)}, 位置:{exp.position})")
                hist_str = "\n  ".join(exp_history) if exp_history else "なし"
                
                failed_warn = ""
                if last_failed_rule:
                    failed_warn = f"\n⚠️ 【重要警告】前回の以下の推論ルールはSandboxで検証に失敗したか、到達不可(-1)でした！別の色をターゲットにするか、壁とする色を減らす等の抜本的な仮説変更が必要です:\n```jcross\n{last_failed_rule}\n```\n"

                prompt = (
                    f"現在スタックしています。これは『世界モデル（仮説）の構築』と『仮説検証のための推論生成』フェーズです。\n"
                    f"{failed_warn}"
                    f"【現在の推論（仮説検証）ルール】\n"
                    f"```jcross\n{current_cortex}\n```\n\n"
                    f"【関係グラフ (Relational Graph Object Abstraction)】\n"
                    f"- 盤面グラフ定義: {identified_patterns}\n"
                    f"- プレイヤー座標: {p_pos}\n\n"
                    f"【これまでの仮説検証の実績 (History)】\n  {hist_str}\n\n"
                    f"上記の関係グラフと過去の行動結果（特に 'blocked' になった色体験）から、このパズルの法則（〇色は壁で通れない等）の推移仮説を論理的に文章で立ててください。\n"
                    f"そして、その仮説をもとに、最新の `経路探索(目標地点の[Y, X]配列, [通れない壁の色の配列])` を使って次の行動を決定する JCross 推論ルールコードを出力してください。\n"
                    f"※ 過去に 'blocked' となった色は必ず壁の配列に含めてください。手動で方向(0〜3)を返すロジックは極力避け、Planner関数の出力(返り値)をそのまま Return してください。\n"
                    f"※ どの条件にも一致しない場合は必ず `返す -1` を記述してください。\n"
                )
                
                response_text = call_gemini_api(prompt, SYSTEM_PROMPT)
                if not response_text:
                    print("Evolution failed (API did not return). Aborting segment.")
                    break
                    
                rule_str = extract_jcross_code(response_text)
                if not rule_str:
                    print("Evolution failed (No JCross code extracted). Aborting segment.")
                    print(f"LLM Response was: {response_text}")
                    break
                    
                print(f"✨ Extracted Rule: {rule_str}")
                
                # Sandbox ロールアウト
                if test_rule_in_sandbox(soul_path, grid_data, p_pos, t_pos, anomalies_summary, phase, rule_str):
                    # Living Cortex Rewrite (Actually already written by Sandbox, but we can formally keep it)
                    agent_brain.rewrite_rule("AI_CORTEX", rule_str)
                    print(f"💾 Surgically updated AI_CORTEX in {soul_path}")
                    
                    # Brain Reload
                    print("🧠 Reloading brain...")
                    agent_brain = load_brain()
                    evolution_attempts = 0
                    
                    # 再試行（同じフレームで決断）
                    print("🔄 Retrying step with new intelligence...")
                    continue
                else:
                    evolution_attempts += 1
                    if evolution_attempts >= 3:
                        print("🔥 Evolution failed 3 times. Discarding and aborting.")
                        break
                    print(f"🔥 New rule failed Sandbox Verification (Attempt {evolution_attempts}/3). Triggering re-think...")
                    last_failed_rule = rule_str
                    continue
                
            print(f"▶️ [{game_name}] Step {actions}: JCross chose action {action_decision}")
            
            try:
                # arcengineのGameActionコンストラクタバグを回避するため、安全なループ検索を使用
                env_action = next((a for a in GameAction if a.value == int(action_decision)), None)
                if env_action is None:
                    raise ValueError(f"Action {action_decision} is not mapped in arcengine.GameAction")
                frame = env.step(env_action)
                
                # ----------------------------------------------------
                # [CrossSpace] 体験の抽出と保存
                # ----------------------------------------------------
                new_grid_data = getattr(frame, "grid", [])
                new_p_pos = []
                if new_grid_data and CrossWorld:
                    new_world = CrossWorld(new_grid_data)
                    if new_world.player_pos:
                        new_p_pos = list(new_world.player_pos)
                        
                exps = extract_experiences(grid_data, new_grid_data, p_pos, new_p_pos, action_decision, actions)
                for exp in exps:
                    cross_space.add(exp)
                    
            except Exception as e:
                print(f"Environment Error: {e}")
                break
                
            actions += 1
            if getattr(frame, "state", "") == "GAME_OVER":
                print(f"-> GAME OVER on step {actions}")
                break
                
        levels = getattr(frame, 'levels_completed', 0)
        total_levels += levels
        print(f"🏁 {game_name} completed: levels={levels} actions={actions}")

    print(f"\nJCross Auto-Evolved Engine total: {total_levels} levels from {len(test_games)} games")

if __name__ == "__main__":
    main()
