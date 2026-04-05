import sys
import os
import json

engine_dir = "/Users/motonishikoudai/verantyx-cli/src/verantyx/cross_engine"
sys.path.insert(0, engine_dir)

from jcross_runtime import JCrossRuntime

def predict(json_str):
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

    for k, v in vars_dict.items():
        sandbox_engine.inject(k, v)

    if virtual_rules:
        sandbox_engine.inject_rules(virtual_rules)

    action = sandbox_engine.decide()
    
    print("====== [JCross Sandbox Prediction Result] ======")
    print(f"Action Decision: {action}")
    print("================================================")

if __name__ == "__main__":
    # Remove script name, join the rest into a single string to bypass `.split(" ")` bridge issues.
    raw_args = " ".join(sys.argv[1:])
    # Strip `--vars` if the LLM happened to use it
    if raw_args.startswith("--vars "):
        raw_args = raw_args[7:]
    
    # Strip any leading/trailing quotes LLM might use
    raw_args = raw_args.strip().strip("'").strip('"')
    
    predict(raw_args)
