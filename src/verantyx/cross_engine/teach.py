#!/usr/bin/env python3
import sys
import os
import json
from datetime import datetime
import uuid

engine_dir = "/Users/motonishikoudai/verantyx-cli/src/verantyx/cross_engine"
soul_path = os.path.join(engine_dir, "soul.jcross")

def teach(json_str):
    try:
        data = json.loads(json_str)
    except json.JSONDecodeError as e:
        print(f"Error: Could not parse JSON array/dict -> {json_str} (err: {e})")
        return

    new_rule = data.get("新規ルール")
    description = data.get("説明", "Web AIによる動的追加ルールの自己適応")

    if not new_rule:
        print("Error: '新規ルール' key is missing in JSON payload.")
        return

    if not os.path.exists(soul_path):
        print(f"Error: soul.jcross not found at {soul_path}")
        return

    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    rule_id = f"動的生成_{timestamp}_{str(uuid.uuid4())[:4]}"

    # soul.jcross の末尾に追記
    rule_block = f"""
// [RULE:{rule_id} START]
// 理由・説明: {description}
{new_rule}
// [RULE:{rule_id} END]
"""

    try:
        with open(soul_path, "a", encoding="utf-8") as f:
            f.write(rule_block)
        print("====== [JCross Teach Module Result] ======")
        print(f"Success! 新規ルールを soul.jcross に永続化しました。")
        print(f"Rule ID: {rule_id}")
        print("==========================================")
    except Exception as e:
        print(f"Error: Failed to append to soul.jcross: {e}")

if __name__ == "__main__":
    # Remove script name, join the rest into a single string
    raw_args = " ".join(sys.argv[1:])
    # Strip `--vars` if the LLM happened to use it
    if raw_args.startswith("--vars "):
        raw_args = raw_args[7:]
    
    # Strip any leading/trailing quotes
    raw_args = raw_args.strip().strip("'").strip('"')
    
    teach(raw_args)
