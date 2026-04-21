#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from datetime import datetime, timezone
from pathlib import Path

import pandas as pd
import requests


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Fetch public hourly market data and derive regime features."
    )
    parser.add_argument(
        "--output-csv",
        default=str(Path(__file__).resolve().parent / "data" / "public_market_hourly.csv"),
    )
    parser.add_argument("--lookback-hours", type=int, default=24 * 60)
    parser.add_argument("--timeout-seconds", type=int, default=20)
    return parser.parse_args()


def fetch_binance_klines(
    symbol: str, interval: str, limit: int, timeout_seconds: int
) -> pd.DataFrame:
    url = "https://api.binance.com/api/v3/klines"
    all_rows: list[dict] = []
    remaining = int(limit)
    end_time_ms: int | None = None

    while remaining > 0:
        batch = min(remaining, 1000)
        params = {"symbol": symbol, "interval": interval, "limit": batch}
        if end_time_ms is not None:
            params["endTime"] = end_time_ms

        resp = requests.get(url, params=params, timeout=timeout_seconds)
        resp.raise_for_status()
        raw = resp.json()
        if not isinstance(raw, list):
            raise RuntimeError(f"Unexpected response for {symbol}: {type(raw)}")
        if not raw:
            break

        rows = [
            {
                "ts": pd.to_datetime(int(k[0]), unit="ms", utc=True),
                "open": float(k[1]),
                "high": float(k[2]),
                "low": float(k[3]),
                "close": float(k[4]),
                "volume": float(k[5]),
            }
            for k in raw
        ]
        all_rows.extend(rows)
        oldest_open_ms = int(raw[0][0])
        end_time_ms = oldest_open_ms - 1
        remaining -= len(raw)
        if len(raw) < batch:
            break

    df = pd.DataFrame(all_rows)
    if df.empty:
        return df
    return df.drop_duplicates(subset=["ts"]).sort_values("ts").reset_index(drop=True)


def derive_features(df: pd.DataFrame, prefix: str) -> pd.DataFrame:
    out = df.copy()
    out[f"{prefix}_ret_1h"] = out["close"].pct_change().fillna(0.0)
    out[f"{prefix}_range_1h"] = ((out["high"] - out["low"]) / out["open"]).replace(
        [float("inf"), float("-inf")], 0.0
    ).fillna(0.0)
    vol_mean = out["volume"].rolling(24, min_periods=4).mean()
    vol_std = out["volume"].rolling(24, min_periods=4).std().replace(0, pd.NA)
    out[f"{prefix}_vol_z24"] = ((out["volume"] - vol_mean) / vol_std).fillna(0.0)
    keep = ["ts", f"{prefix}_ret_1h", f"{prefix}_range_1h", f"{prefix}_vol_z24"]
    return out[keep]


def main() -> None:
    args = parse_args()
    out_path = Path(args.output_csv)
    out_path.parent.mkdir(parents=True, exist_ok=True)

    limit = max(args.lookback_hours + 48, 240)

    btc = fetch_binance_klines("BTCUSDT", "1h", limit, args.timeout_seconds)
    eth = fetch_binance_klines("ETHUSDT", "1h", limit, args.timeout_seconds)

    btc_f = derive_features(btc, "btc")
    eth_f = derive_features(eth, "eth")
    merged = btc_f.merge(eth_f, on="ts", how="inner")

    # Rolling 24h cross-asset correlation on returns.
    merged["btc_eth_corr_24"] = (
        merged["btc_ret_1h"]
        .rolling(24, min_periods=8)
        .corr(merged["eth_ret_1h"])
        .fillna(0.0)
    )
    merged["market_stress_1h"] = (
        merged["btc_range_1h"] + merged["eth_range_1h"]
    ) * 0.5 + (merged["btc_vol_z24"].abs() + merged["eth_vol_z24"].abs()) * 0.25

    merged = merged.sort_values("ts").reset_index(drop=True)
    merged = merged.tail(args.lookback_hours).copy()
    merged = merged.rename(columns={"ts": "window_hour_utc"})
    merged.to_csv(out_path, index=False)

    print(
        json.dumps(
            {
                "output_csv": str(out_path),
                "rows": int(len(merged)),
                "from": merged["window_hour_utc"].iloc[0].isoformat() if not merged.empty else None,
                "to": merged["window_hour_utc"].iloc[-1].isoformat() if not merged.empty else None,
                "fetched_at_utc": datetime.now(timezone.utc).isoformat(),
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
