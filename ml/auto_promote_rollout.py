#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path
from urllib.request import urlopen


def parse_args() -> argparse.Namespace:
    repo_root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description="Auto-promote rollout mode based on ml/decision.")
    parser.add_argument("--base-url", default="http://127.0.0.1:8080")
    parser.add_argument("--hours", type=int, default=336)
    parser.add_argument("--env-path", default=str(repo_root / ".env"))
    parser.add_argument("--dry-run", action="store_true")
    return parser.parse_args()


def get_json(url: str) -> dict:
    with urlopen(url, timeout=15) as r:
        return json.loads(r.read().decode("utf-8"))


def update_env_mode(env_path: Path, mode: str) -> None:
    lines = env_path.read_text(encoding="utf-8").splitlines()
    out = []
    found = False
    for line in lines:
        if line.startswith("ML_ROLLOUT_MODE="):
            out.append(f"ML_ROLLOUT_MODE={mode}")
            found = True
        else:
            out.append(line)
    if not found:
        out.append(f"ML_ROLLOUT_MODE={mode}")
    env_path.write_text("\n".join(out) + "\n", encoding="utf-8")


def main() -> None:
    args = parse_args()
    repo_root = Path(__file__).resolve().parents[1]
    decision = get_json(f"{args.base_url}/api/v1/ml/decision?hours={args.hours}")
    health = get_json(f"{args.base_url}/api/v1/ml/health")

    current_mode = health.get("rollout_mode", "shadow")
    recommended_mode = decision.get("recommended_mode", current_mode)
    recommendation = decision.get("recommendation", "")

    action = "none"
    if recommendation == "promote_to_hybrid" and current_mode == "shadow":
        action = "promote_to_hybrid"
    elif recommendation == "promote_to_ml" and current_mode == "hybrid":
        action = "promote_to_ml"

    result = {
        "current_mode": current_mode,
        "recommended_mode": recommended_mode,
        "recommendation": recommendation,
        "action": action,
    }

    if action == "none":
        print(json.dumps(result, indent=2))
        return

    if args.dry_run:
        result["dry_run"] = True
        print(json.dumps(result, indent=2))
        return

    update_env_mode(Path(args.env_path), recommended_mode)
    subprocess.check_call(["docker", "compose", "up", "-d", "backend"], cwd=repo_root)
    result["updated_mode"] = recommended_mode
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
