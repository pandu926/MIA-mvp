from __future__ import annotations

from datetime import datetime, timezone
from pathlib import Path

MODEL_FAMILY = "lightgbm_similarity_iforest"
HORIZONS = (1, 6, 24)
LABEL_ORDER = [
    "breakout_candidate",
    "healthy_rotation",
    "thin_momentum",
    "pump_and_fade",
    "coordinated_distribution",
    "rug_prone_pattern",
]
LABEL_SUMMARIES = {
    "breakout_candidate": "historical analogs point to strong continuation if breadth keeps widening.",
    "healthy_rotation": "historical analogs point to a tradeable rotation instead of a one-leg blow-off.",
    "thin_momentum": "historical analogs point to upside that stays narrow and can fail without fresh participation.",
    "pump_and_fade": "historical analogs point to early strength that usually fades as flow cools.",
    "coordinated_distribution": "historical analogs point to coordinated distribution risk across overlapping wallets.",
    "rug_prone_pattern": "historical analogs point to very weak follow-through and fast failure risk.",
}
FEATURE_COLUMNS = [
    "risk_score",
    "legacy_alpha_score",
    "legacy_rank",
    "baseline_volume_1h",
    "baseline_buys_1h",
    "baseline_sells_1h",
    "buy_share_1h",
    "active_wallets_1h",
    "tx_count_1h",
    "whale_watch_24h",
    "whale_critical_24h",
    "token_age_minutes",
    "initial_liquidity_bnb",
    "deployer_total_tokens",
    "deployer_rug_count",
    "deployer_graduated_count",
    "probable_cluster_wallets",
    "potential_cluster_wallets",
]


def default_model_version(prefix: str = "pattern-engine") -> str:
    stamp = datetime.now(timezone.utc).strftime("%Y%m%d-%H%M%S")
    return f"{prefix}-{stamp}"


def ensure_output_dir(raw: str | Path) -> Path:
    path = Path(raw).expanduser().resolve()
    path.mkdir(parents=True, exist_ok=True)
    return path


def artifact_path(output_dir: str | Path, model_version: str) -> Path:
    return ensure_output_dir(output_dir) / f"{model_version}.joblib"


def metadata_path(output_dir: str | Path, model_version: str) -> Path:
    return ensure_output_dir(output_dir) / f"{model_version}.metadata.json"


def clamp(value: float, lower: float = 0.0, upper: float = 1.0) -> float:
    return max(lower, min(upper, value))

