from __future__ import annotations

import json
from dataclasses import dataclass

import joblib
import numpy as np
import pandas as pd
from lightgbm import LGBMClassifier
from sklearn.dummy import DummyClassifier
from sklearn.ensemble import IsolationForest
from sklearn.metrics import accuracy_score, f1_score
from sklearn.neighbors import NearestNeighbors
from sklearn.preprocessing import StandardScaler

from .common import FEATURE_COLUMNS, LABEL_ORDER, LABEL_SUMMARIES, clamp


@dataclass
class HorizonTrainingResult:
    horizon_hours: int
    sample_count: int
    label_distribution: dict[str, int]
    metrics: dict[str, float | int | str]
    artifact: dict


def derive_outcome_label(row: pd.Series) -> str:
    baseline_volume = max(float(row["baseline_volume_1h"]), 0.05)
    future_volume_ratio = float(row["future_volume"]) / baseline_volume
    future_tx_count = float(row["future_tx_count"])
    total_side_flow = float(row["future_buys"]) + float(row["future_sells"])
    future_buy_share = (float(row["future_buys"]) / total_side_flow) if total_side_flow > 0 else 0.5
    probable_clusters = float(row["probable_cluster_wallets"])

    if future_tx_count <= 2 and future_volume_ratio <= 0.35:
        return "rug_prone_pattern"
    if future_buy_share <= 0.35 and (future_volume_ratio >= 0.9 or probable_clusters >= 2):
        return "coordinated_distribution"
    if future_volume_ratio >= 2.4 and future_buy_share >= 0.62 and future_tx_count >= 8:
        return "breakout_candidate"
    if future_volume_ratio >= 1.4 and 0.52 <= future_buy_share < 0.62 and future_tx_count >= 6:
        return "healthy_rotation"
    if future_volume_ratio >= 0.9 and future_buy_share >= 0.55:
        return "thin_momentum"
    return "pump_and_fade"


def fit_horizon_model(frame: pd.DataFrame, horizon_hours: int) -> HorizonTrainingResult:
    labeled = frame.copy()
    labeled["label"] = labeled.apply(derive_outcome_label, axis=1)
    labeled = labeled[labeled["label"].isin(LABEL_ORDER)].copy()
    if labeled.empty:
        raise RuntimeError(f"No usable training rows for horizon {horizon_hours}H")

    labeled = labeled.sort_values(["window_end", "token_address"]).reset_index(drop=True)
    feature_frame = labeled[FEATURE_COLUMNS].astype(float).fillna(0.0)
    y = labeled["label"].astype(str)

    split_index = max(int(len(labeled) * 0.8), 1)
    split_index = min(split_index, len(labeled) - 1) if len(labeled) > 1 else len(labeled)
    train_x = feature_frame.iloc[:split_index]
    test_x = feature_frame.iloc[split_index:]
    train_y = y.iloc[:split_index]
    test_y = y.iloc[split_index:]

    scaler = StandardScaler()
    scaled_train = scaler.fit_transform(train_x)
    scaled_all = scaler.transform(feature_frame)

    if train_y.nunique() >= 2 and len(train_x) >= 40:
        model = LGBMClassifier(
            objective="multiclass",
            num_class=int(train_y.nunique()),
            n_estimators=120,
            learning_rate=0.05,
            num_leaves=31,
            min_child_samples=12,
            subsample=0.9,
            colsample_bytree=0.9,
            reg_alpha=0.2,
            reg_lambda=1.0,
            n_jobs=2,
            random_state=42,
            verbose=-1,
        )
        model.fit(train_x, train_y)
        model_kind = "lightgbm"
    else:
        model = DummyClassifier(strategy="most_frequent")
        model.fit(train_x, train_y)
        model_kind = "dummy"

    neighbors = NearestNeighbors(n_neighbors=min(8, len(labeled)), metric="euclidean")
    neighbors.fit(scaled_all)

    anomaly_model = None
    if len(labeled) >= 64:
        anomaly_model = IsolationForest(
            n_estimators=160,
            contamination=0.08,
            random_state=42,
        )
        anomaly_model.fit(scaled_all)

    predictions = model.predict(test_x) if len(test_x) > 0 else model.predict(train_x)
    truth = test_y if len(test_y) > 0 else train_y
    accuracy = float(accuracy_score(truth, predictions)) if len(truth) > 0 else 0.0
    macro_f1 = float(f1_score(truth, predictions, average="macro")) if len(truth) > 0 else 0.0

    reference_rows = []
    for _, row in labeled.iterrows():
        reference_rows.append(
            {
                "token_address": row["token_address"],
                "window_end": pd.Timestamp(row["window_end"]).isoformat(),
                "match_label": row["label"],
                "outcome_class": row["label"],
                "rationale": LABEL_SUMMARIES[row["label"]],
                "features": {name: float(row[name]) for name in FEATURE_COLUMNS},
            }
        )

    label_distribution = y.value_counts().sort_index().to_dict()
    metadata = {
        "horizon_hours": horizon_hours,
        "feature_columns": FEATURE_COLUMNS,
        "label_order": LABEL_ORDER,
        "model_kind": model_kind,
        "label_distribution": label_distribution,
        "accuracy": accuracy,
        "macro_f1": macro_f1,
        "sample_count": int(len(labeled)),
        "train_rows": int(len(train_x)),
        "test_rows": int(len(test_x)),
    }
    artifact = {
        "metadata": metadata,
        "model": model,
        "scaler": scaler,
        "neighbors": neighbors,
        "anomaly_model": anomaly_model,
        "reference_rows": reference_rows,
    }
    return HorizonTrainingResult(
        horizon_hours=horizon_hours,
        sample_count=int(len(labeled)),
        label_distribution=label_distribution,
        metrics={
            "accuracy": accuracy,
            "macro_f1": macro_f1,
            "sample_count": int(len(labeled)),
            "model_kind": model_kind,
        },
        artifact=artifact,
    )


def save_artifact_bundle(path, bundle: dict) -> None:
    joblib.dump(bundle, path)


def load_artifact_bundle(path):
    return joblib.load(path)


def _normalize_anomaly(anomaly_model: IsolationForest | None, scaled_row: np.ndarray) -> float:
    if anomaly_model is None:
        return 0.15
    decision = float(anomaly_model.decision_function(scaled_row.reshape(1, -1))[0])
    return clamp(1.0 / (1.0 + np.exp(decision * 4.0)))


def _build_notable_differences(features: dict, analog_features: dict) -> list[str]:
    highlights = []
    tracked = [
        "baseline_volume_1h",
        "buy_share_1h",
        "active_wallets_1h",
        "deployer_rug_count",
        "probable_cluster_wallets",
    ]
    for key in tracked:
        current = float(features.get(key, 0.0))
        analog = float(analog_features.get(key, 0.0))
        if key == "buy_share_1h":
            diff = abs(current - analog)
            if diff >= 0.18:
                highlights.append(f"{key} differs by {diff:.2f}")
        else:
            baseline = max(abs(analog), 1.0)
            delta = abs(current - analog) / baseline
            if delta >= 0.4:
                highlights.append(f"{key} differs by {delta * 100:.0f}%")
        if len(highlights) >= 2:
            break
    return highlights


def _build_analogs(horizon_artifact: dict, scaled_row: np.ndarray, feature_values: dict) -> tuple[list[dict], float]:
    distances, indices = horizon_artifact["neighbors"].kneighbors(scaled_row.reshape(1, -1))
    analogs = []
    top_similarity = 0.0
    for distance, index in zip(distances[0][:3], indices[0][:3]):
        ref = horizon_artifact["reference_rows"][int(index)]
        match_score = clamp(1.0 / (1.0 + float(distance)))
        top_similarity = max(top_similarity, match_score)
        analogs.append(
            {
                "token_address": ref["token_address"],
                "window_end": ref["window_end"],
                "match_score": round(match_score, 4),
                "match_label": ref["match_label"],
                "outcome_class": ref["outcome_class"],
                "rationale": ref["rationale"],
                "notable_differences": _build_notable_differences(
                    feature_values,
                    ref.get("features", {}),
                ),
            }
        )
    return analogs, top_similarity


def predict_horizon(horizon_artifact: dict, frame: pd.DataFrame, horizon_hours: int) -> list[dict]:
    model = horizon_artifact["model"]
    scaler = horizon_artifact["scaler"]
    feature_frame = frame[FEATURE_COLUMNS].astype(float).fillna(0.0)
    scaled = scaler.transform(feature_frame)
    probabilities = model.predict_proba(feature_frame)
    classes = list(getattr(model, "classes_", LABEL_ORDER))

    predictions = []
    for index, (_, row) in enumerate(frame.iterrows()):
        feature_values = {name: float(row[name]) for name in FEATURE_COLUMNS}
        row_probs = probabilities[index]
        if isinstance(row_probs, np.ndarray) and row_probs.ndim > 0:
            best_index = int(np.argmax(row_probs))
            predicted_label = str(classes[best_index])
            predicted_score = float(row_probs[best_index])
        else:
            predicted_label = str(classes[0])
            predicted_score = 1.0
        analogs, top_similarity = _build_analogs(horizon_artifact, scaled[index], feature_values)
        anomaly_score = _normalize_anomaly(horizon_artifact.get("anomaly_model"), scaled[index])
        confidence = clamp((predicted_score * 0.6) + (top_similarity * 0.3) + ((1.0 - anomaly_score) * 0.1))
        top_analog = analogs[0] if analogs else None
        rationale = (
            f"{horizon_hours}H model leans {predicted_label.replace('_', ' ')}. "
            f"Baseline volume is {feature_values['baseline_volume_1h']:.2f} BNB with "
            f"{feature_values['active_wallets_1h']:.0f} active wallets and "
            f"{feature_values['probable_cluster_wallets']:.0f} probable cluster wallets."
        )
        if top_analog:
            rationale += (
                f" Closest analog is {top_analog['token_address']} at "
                f"{top_analog['match_score'] * 100:.0f}% similarity."
            )

        predictions.append(
            {
                "window_end": row["window_end"],
                "token_address": row["token_address"],
                "horizon_hours": horizon_hours,
                "match_label": predicted_label,
                "outcome_class": predicted_label,
                "score": round(predicted_score, 4),
                "confidence": round(confidence, 4),
                "anomaly_score": round(anomaly_score, 4),
                "expected_path_summary": LABEL_SUMMARIES[predicted_label],
                "rationale": rationale,
                "analogs": analogs,
                "feature_snapshot": {
                    **feature_values,
                    "top_analog_similarity": round(top_similarity, 4),
                },
            }
        )
    return predictions


def write_metadata(path, payload: dict) -> None:
    path.write_text(json.dumps(payload, indent=2), encoding="utf-8")
