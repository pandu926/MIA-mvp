# MIA ML Pipeline (Alpha Ranking)

This folder contains the ML and pattern-engine code that supports the MIA MVP.
It intentionally keeps source code and automation scripts only. Large datasets,
venvs, logs, and exported artifacts are excluded from the public repository.

## Setup

```bash
cd /path/to/MIA-mvp
python3 -m venv .venv-ml
source .venv-ml/bin/activate
pip install -r ml/requirements.txt
```

Set the required runtime values before using the training or inference scripts:

- `DATABASE_URL`
- optionally `LOCAL_DATABASE_URL` for host-side jobs
- optionally `VENV_PYTHON` or `VENV_ACTIVATE` if you use a non-default venv path

## 1) Train Model

Fetch external public market features first:

```bash
python ml/fetch_public_market_data.py --lookback-hours 720
```

```bash
python ml/train_alpha_model.py \
  --lookback-hours 336 \
  --model-version lightgbm-v1 \
  --rollout-mode shadow \
  --activate
```

Output:
- artifact model: `backend/ml_models/<model_version>.joblib`
- metadata: `backend/ml_models/<model_version>.metadata.json`
- registry row di `ml_model_registry`
- model terkalibrasi (isotonic) + baseline logistic tersimpan di artifact
- walk-forward metrics tersimpan di metadata
- jika tersedia, fitur public market regime otomatis digabungkan (`ml/data/public_market_hourly.csv`)

## 2) Run Inference

```bash
python ml/run_alpha_inference.py --model-version lightgbm-v1
```

Perintah ini menulis prediksi `score_source='ml'` ke tabel `ml_alpha_predictions` untuk window alpha terbaru.

Dry-run:

```bash
python ml/run_alpha_inference.py --model-version lightgbm-v1 --dry-run
```

## 3) API Monitoring

- `GET /api/v1/ml/health`
- `GET /api/v1/ml/alpha/eval?hours=24`
- `GET /api/v1/ml/decision?hours=168`
- `GET /api/v1/ml/models`
- `POST /api/v1/ml/models` (activate / update rollout metadata)

Contoh activate via API:

```json
{
  "model_version": "lightgbm-v1",
  "rollout_mode": "hybrid",
  "activate": true
}
```

## 4) Automated Jobs

- inference hourly: `ml/run_inference_hourly.sh`
- realized-label update hourly: `ml/run_label_update_hourly.sh`
- model backup daily: `ml/backup_latest_model.sh`
- monitor freshness (alerts via syslog/logger): `ml/check_ml_jobs.sh`
- auto promotion gate: `ml/run_auto_promotion.sh`

These scripts are repo-relative and designed to work after a normal clone.
They no longer assume a fixed absolute repo path.

## 5) Remote Pattern-Engine Training

Pattern-engine training can run on a separate high-spec VPS without touching the main project folder there.

Required environment variables:

```bash
export MIA_REMOTE_HOST=your-remote-host
export MIA_REMOTE_USER=root
export MIA_REMOTE_PASSWORD='your-password'
export LOCAL_DATABASE_URL='postgres://mia:mia_password@localhost:5432/mia_db'
```

Sync ML workspace to the isolated remote folder:

```bash
./ml/remote_pattern_sync.sh
```

Run remote training in the isolated folder:

```bash
MODEL_VERSION=pattern-engine-remote-test \
LOOKBACK_HOURS=168 \
./ml/remote_pattern_train.sh
```

Pull the trained artifact back to the local repo:

```bash
MODEL_VERSION=pattern-engine-remote-test \
./ml/remote_pattern_pull.sh
```

## 6) Historical Pattern Backfill

The rolling exporter uses recent `alpha_rankings`, which only covers the live window history. For the historical pattern corpus, use the dedicated token-based backfill path instead.

Default historical scope:

- window anchor: `token.deployed_at + 1 hour`
- range: `2025-04-01` to `2026-05-01`
- eligibility: `lifetime tx count >= 50`

Preview the batch size without writing CSV files:

```bash
SUMMARY_ONLY=1 ./ml/run_historical_pattern_backfill.sh
```

Export the historical CSVs:

```bash
./ml/run_historical_pattern_backfill.sh
```

Override the defaults when needed:

```bash
START_AT=2025-06-01T00:00:00Z \
END_AT=2026-01-01T00:00:00Z \
MIN_TOTAL_TX=150 \
HORIZONS=1,6,24 \
./ml/run_historical_pattern_backfill.sh
```

Run remote training from the historical corpus:

```bash
export MIA_REMOTE_HOST=your-remote-host
export MIA_REMOTE_USER=root
export MIA_REMOTE_PASSWORD='your-password'
export LOCAL_DATABASE_URL='postgres://mia:mia_password@localhost:5432/mia_db'

MODEL_VERSION=pattern-engine-historical-v1 \
HISTORICAL_START_AT=2025-04-01T00:00:00Z \
HISTORICAL_END_AT=2026-05-01T00:00:00Z \
MIN_TOTAL_TX=50 \
./ml/remote_pattern_train.sh
```
